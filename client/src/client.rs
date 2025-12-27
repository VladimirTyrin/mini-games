use common::menu_service_client::MenuServiceClient;
use common::game_service_client::GameServiceClient;
use common::{MenuClientMessage, GameClientMessage, ConnectRequest, DisconnectRequest, ListLobbiesRequest, CreateLobbyRequest, JoinLobbyRequest, LeaveLobbyRequest, MarkReadyRequest, StartGameRequest, LobbySettings, log, TurnCommand};
use tokio::sync::mpsc;
use crate::state::{MenuCommand, GameCommand, SharedState, AppState};
use crate::config::{ConfigManager, FileContentConfigProvider, Config, YamlConfigSerializer};

#[derive(Clone)]
pub struct GrpcLoggingSender<T> {
    inner: mpsc::Sender<T>,
}

impl<T> GrpcLoggingSender<T>
where
    T: std::fmt::Debug + prost::Message
{
    pub fn new(inner: mpsc::Sender<T>) -> Self {
        Self { inner }
    }

    pub async fn send(&self, value: T) -> Result<(), mpsc::error::SendError<T>> {
        log!("Sending: {:?}", value);
        self.inner.send(value).await
    }
}

pub async fn grpc_client_task(
    client_id: String,
    mut server_address: String,
    shared_state: SharedState,
    mut command_rx: mpsc::UnboundedReceiver<MenuCommand>,
    config_manager: ConfigManager<FileContentConfigProvider, Config, YamlConfigSerializer>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let mut menu_client = match MenuServiceClient::connect(server_address.clone()).await {
            Ok(client) => {
                let mut config = config_manager.get_config().unwrap_or_default();
                config.server.address = server_address.clone();
                let _ = config_manager.set_config(&config);
                shared_state.set_connection_failed(false);
                client
            },
            Err(e) => {
                shared_state.set_connection_failed(true);
                shared_state.set_error(format!("Failed to connect to server at {}: {}", server_address, e));

                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    if let Some(new_address) = shared_state.take_retry_server_address() {
                        server_address = new_address;
                        break;
                    }

                    if shared_state.should_close() {
                        return Err(e.into());
                    }
                }
                continue;
            }
        };

    let (tx_raw, rx) = mpsc::channel(128);
    let tx = GrpcLoggingSender::new(tx_raw);

    let menu_stream = menu_client
        .menu_stream(tokio_stream::wrappers::ReceiverStream::new(rx))
        .await?;
    let mut response_stream = menu_stream.into_inner();

    tx.send(MenuClientMessage {
        message: Some(common::menu_client_message::Message::Connect(ConnectRequest {
            client_id: client_id.clone(),
        })),
    })
    .await?;

    tx.send(MenuClientMessage {
        message: Some(common::menu_client_message::Message::ListLobbies(ListLobbiesRequest {})),
    })
    .await?;

    loop {
        tokio::select! {
            Some(command) = command_rx.recv() => {
                let message = match command {
                    MenuCommand::ListLobbies => {
                        Some(common::menu_client_message::Message::ListLobbies(ListLobbiesRequest {}))
                    }
                    MenuCommand::CreateLobby { name, config } => {
                        Some(common::menu_client_message::Message::CreateLobby(CreateLobbyRequest {
                            lobby_name: name,
                            max_players: config.max_players,
                            settings: Some(LobbySettings {
                                field_width: config.field_width,
                                field_height: config.field_height,
                                wall_collision_mode: config.wall_collision_mode.into(),
                                tick_interval_ms: config.tick_interval_ms,
                            }),
                        }))
                    }
                    MenuCommand::JoinLobby { lobby_id } => {
                        Some(common::menu_client_message::Message::JoinLobby(JoinLobbyRequest {
                            lobby_id,
                        }))
                    }
                    MenuCommand::LeaveLobby => {
                        Some(common::menu_client_message::Message::LeaveLobby(LeaveLobbyRequest {}))
                    }
                    MenuCommand::MarkReady { ready } => {
                        Some(common::menu_client_message::Message::MarkReady(MarkReadyRequest {
                            ready,
                        }))
                    }
                    MenuCommand::StartGame => {
                        Some(common::menu_client_message::Message::StartGame(StartGameRequest {}))
                    }
                    MenuCommand::Disconnect => {
                        let _ = tx.send(MenuClientMessage {
                            message: Some(common::menu_client_message::Message::Disconnect(DisconnectRequest {})),
                        }).await;
                        break;
                    }
                };

                if let Some(msg) = message {
                    if tx.send(MenuClientMessage {
                        message: Some(msg),
                    }).await.is_err() {
                        break;
                    }
                }
            }

            result = response_stream.message() => {
                match result {
                    Ok(Some(server_msg)) => {
                        log!("Received: {:?}", server_msg);

                        if let Some(msg) = server_msg.message {
                            match msg {
                                common::menu_server_message::Message::LobbyList(list) => {
                                    shared_state.set_state(AppState::LobbyList {
                                        lobbies: list.lobbies,
                                    });
                                }
                                common::menu_server_message::Message::LobbyUpdate(update) => {
                                    if let Some(details) = update.lobby {
                                        let current_state = shared_state.get_state();
                                        match current_state {
                                            AppState::InLobby { event_log, .. } => {
                                                shared_state.set_state(AppState::InLobby {
                                                    details,
                                                    event_log,
                                                });
                                            }
                                            _ => {
                                                shared_state.set_state(AppState::InLobby {
                                                    details,
                                                    event_log: vec![],
                                                });
                                            }
                                        }
                                    }
                                }
                                common::menu_server_message::Message::PlayerJoined(notification) => {
                                    shared_state.add_event(format!("{} joined", notification.client_id));
                                }
                                common::menu_server_message::Message::PlayerLeft(notification) => {
                                    shared_state.add_event(format!("{} left", notification.client_id));
                                }
                                common::menu_server_message::Message::PlayerReady(notification) => {
                                    if notification.ready {
                                        shared_state.add_event(format!("{} is ready", notification.client_id));
                                    } else {
                                        shared_state.add_event(format!("{} is not ready anymore", notification.client_id));
                                    }
                                }
                                common::menu_server_message::Message::LobbyListUpdate(_) => {
                                    let current_state = shared_state.get_state();
                                    if matches!(current_state, AppState::LobbyList { .. }) {
                                        if tx.send(MenuClientMessage {
                                            message: Some(common::menu_client_message::Message::ListLobbies(
                                                ListLobbiesRequest {}
                                            )),
                                        }).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                                common::menu_server_message::Message::Error(err) => {
                                    shared_state.set_error(err.message);
                                }
                                common::menu_server_message::Message::ServerShuttingDown(notification) => {
                                    shared_state.set_error(notification.message);
                                    shared_state.set_should_close();
                                    break;
                                }
                                common::menu_server_message::Message::LobbyClosed(notification) => {
                                    shared_state.set_error(notification.message);
                                    if tx.send(MenuClientMessage {
                                        message: Some(common::menu_client_message::Message::ListLobbies(
                                            ListLobbiesRequest {}
                                        )),
                                    }).await.is_err() {
                                        break;
                                    }
                                }
                                common::menu_server_message::Message::GameStarting(notification) => {
                                    log!("Game starting! Session ID: {}", notification.session_id);
                                    shared_state.set_state(AppState::InGame {
                                        session_id: notification.session_id.clone(),
                                        game_state: None,
                                    });

                                    let server_addr = server_address.clone();
                                    let shared_state_clone = shared_state.clone();
                                    let client_id_clone = client_id.clone();

                                    tokio::spawn(async move {
                                        if let Err(e) = game_client_task(
                                            client_id_clone,
                                            server_addr,
                                            shared_state_clone,
                                        ).await {
                                            log!("Game client error: {}", e);
                                        }
                                    });
                                }
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        shared_state.set_error(format!("Connection error: {}", e));
                        shared_state.set_should_close();
                        break;
                    }
                }
            }
        }
    }

    let _ = tx.send(MenuClientMessage {
        message: Some(common::menu_client_message::Message::Disconnect(DisconnectRequest {})),
    }).await;

    break;
    }

    Ok(())
}

async fn game_client_task(
    client_id: String,
    server_address: String,
    shared_state: SharedState,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut game_client = GameServiceClient::connect(server_address).await?;

    let (tx_raw, rx) = mpsc::channel(128);
    let tx = GrpcLoggingSender::new(tx_raw);

    let (game_command_tx, mut game_command_rx) = mpsc::unbounded_channel::<GameCommand>();
    shared_state.set_game_command_tx(game_command_tx);

    let game_stream = game_client
        .game_stream(tokio_stream::wrappers::ReceiverStream::new(rx))
        .await?;
    let mut response_stream = game_stream.into_inner();

    tx.send(GameClientMessage {
        message: Some(common::game_client_message::Message::Connect(ConnectRequest {
            client_id: client_id.clone(),
        })),
    })
    .await?;

    loop {
        tokio::select! {
            Some(command) = game_command_rx.recv() => {
                match command {
                    GameCommand::SendTurn { direction } => {
                        if tx.send(GameClientMessage {
                            message: Some(common::game_client_message::Message::Turn(TurnCommand {
                                direction: direction as i32,
                            })),
                        }).await.is_err() {
                            break;
                        }
                    }
                }
            }

            result = response_stream.message() => {
                match result {
                    Ok(Some(server_msg)) => {
                        log!("Game: Received: {:?}", server_msg);

                        if let Some(msg) = server_msg.message {
                            match msg {
                                common::game_server_message::Message::State(state) => {
                                    shared_state.update_game_state(state);
                                }
                                common::game_server_message::Message::GameOver(game_over) => {
                                    log!("Game over! Winner: {}", game_over.winner_id);

                                    let last_game_state = match shared_state.get_state() {
                                        AppState::InGame { game_state, .. } => game_state,
                                        _ => None,
                                    };

                                    let reason = common::GameEndReason::try_from(game_over.reason)
                                        .unwrap_or(common::GameEndReason::Unspecified);

                                    shared_state.set_state(AppState::GameOver {
                                        scores: game_over.scores,
                                        winner_id: game_over.winner_id,
                                        last_game_state,
                                        reason,
                                    });
                                    break;
                                }
                                common::game_server_message::Message::Error(err) => {
                                    log!("Game error: {}", err.message);
                                    shared_state.set_error(err.message);
                                    break;
                                }
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        log!("Game connection error: {}", e);
                        shared_state.set_error(format!("Game connection error: {}", e));
                        break;
                    }
                }
            }
        }
    }

    log!("Game client task ending");
    shared_state.clear_game_command_tx();
    Ok(())
}
