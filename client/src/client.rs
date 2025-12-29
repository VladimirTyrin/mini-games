use common::snake_game_service_client::SnakeGameServiceClient;
use common::{ClientMessage, client_message, ConnectRequest, DisconnectRequest, ListLobbiesRequest, CreateLobbyRequest, JoinLobbyRequest, LeaveLobbyRequest, MarkReadyRequest, StartGameRequest, PlayAgainRequest, LobbySettings, log, TurnCommand};
use tokio::sync::mpsc;
use crate::state::{MenuCommand, GameCommand, ClientCommand, SharedState, AppState, PlayAgainStatus};
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
    mut command_rx: mpsc::UnboundedReceiver<ClientCommand>,
    config_manager: ConfigManager<FileContentConfigProvider, Config, YamlConfigSerializer>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let mut client = match SnakeGameServiceClient::connect(server_address.clone()).await {
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

    let stream = client
        .game_stream(tokio_stream::wrappers::ReceiverStream::new(rx))
        .await?;
    let mut response_stream = stream.into_inner();

    tx.send(ClientMessage {
        message: Some(client_message::Message::Connect(ConnectRequest {
            client_id: client_id.clone(),
        })),
    })
    .await?;

    tx.send(ClientMessage {
        message: Some(client_message::Message::ListLobbies(ListLobbiesRequest {})),
    })
    .await?;

    loop {
        tokio::select! {
            Some(command) = command_rx.recv() => {
                let message = match command {
                    ClientCommand::Menu(menu_cmd) => {
                        match menu_cmd {
                            MenuCommand::ListLobbies => {
                                Some(client_message::Message::ListLobbies(ListLobbiesRequest {}))
                            }
                            MenuCommand::CreateLobby { name, config } => {
                                Some(client_message::Message::CreateLobby(CreateLobbyRequest {
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
                                Some(client_message::Message::JoinLobby(JoinLobbyRequest {
                                    lobby_id,
                                }))
                            }
                            MenuCommand::LeaveLobby => {
                                Some(client_message::Message::LeaveLobby(LeaveLobbyRequest {}))
                            }
                            MenuCommand::MarkReady { ready } => {
                                Some(client_message::Message::MarkReady(MarkReadyRequest {
                                    ready,
                                }))
                            }
                            MenuCommand::StartGame => {
                                Some(client_message::Message::StartGame(StartGameRequest {}))
                            }
                            MenuCommand::PlayAgain => {
                                Some(client_message::Message::PlayAgain(PlayAgainRequest {}))
                            }
                            MenuCommand::Disconnect => {
                                Some(client_message::Message::Disconnect(DisconnectRequest {}))
                            }
                        }
                    }
                    ClientCommand::Game(game_cmd) => {
                        match game_cmd {
                            GameCommand::SendTurn { direction } => {
                                Some(client_message::Message::Turn(TurnCommand {
                                    direction: direction as i32,
                                }))
                            }
                        }
                    }
                };

                if let Some(msg) = message {
                    if tx.send(ClientMessage { message: Some(msg) }).await.is_err() {
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
                                common::server_message::Message::LobbyList(lobby_list) => {
                                    // Only update lobby list if we're currently in the lobby list state
                                    // Don't transition if we're in a lobby, game, or game over
                                    if matches!(shared_state.get_state(), AppState::LobbyList { .. }) {
                                        shared_state.set_state(AppState::LobbyList {
                                            lobbies: lobby_list.lobbies,
                                        });
                                    }
                                }
                                common::server_message::Message::LobbyUpdate(update) => {
                                    if let Some(lobby) = update.lobby {
                                        shared_state.set_state(AppState::InLobby {
                                            details: lobby,
                                            event_log: Vec::new(),
                                        });
                                    }
                                }
                                common::server_message::Message::PlayerJoined(notification) => {
                                    shared_state.add_event_log(format!("{} joined", notification.client_id));
                                }
                                common::server_message::Message::PlayerLeft(notification) => {
                                    shared_state.add_event_log(format!("{} left", notification.client_id));
                                }
                                common::server_message::Message::PlayerReady(notification) => {
                                    let status = if notification.ready { "ready" } else { "not ready" };
                                    shared_state.add_event_log(format!("{} is {}", notification.client_id, status));
                                }
                                common::server_message::Message::Error(err) => {
                                    shared_state.set_error(err.message);
                                }
                                common::server_message::Message::LobbyListUpdate(_) => {
                                    if tx.send(ClientMessage {
                                        message: Some(client_message::Message::ListLobbies(
                                            ListLobbiesRequest {}
                                        )),
                                    }).await.is_err() {
                                        break;
                                    }
                                }
                                common::server_message::Message::ServerShuttingDown(_) => {
                                    shared_state.set_error("Server is shutting down".to_string());
                                    shared_state.set_should_close();
                                    break;
                                }
                                common::server_message::Message::LobbyClosed(notification) => {
                                    shared_state.set_error(notification.message);
                                    if tx.send(ClientMessage {
                                        message: Some(client_message::Message::ListLobbies(
                                            ListLobbiesRequest {}
                                        )),
                                    }).await.is_err() {
                                        break;
                                    }
                                }
                                common::server_message::Message::GameStarting(notification) => {
                                    log!("Game starting! Session ID: {}", notification.session_id);
                                    shared_state.set_state(AppState::InGame {
                                        session_id: notification.session_id.clone(),
                                        game_state: None,
                                    });
                                }
                                common::server_message::Message::State(state) => {
                                    shared_state.update_game_state(state);
                                }
                                common::server_message::Message::GameOver(game_over) => {
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
                                        play_again_status: PlayAgainStatus::NotAvailable,
                                    });
                                }
                                common::server_message::Message::PlayAgainStatus(notification) => {
                                    if let Some(status) = notification.status {
                                        let play_again_status = match status {
                                            common::play_again_status_notification::Status::NotAvailable(_) => {
                                                PlayAgainStatus::NotAvailable
                                            }
                                            common::play_again_status_notification::Status::Available(available) => {
                                                PlayAgainStatus::Available {
                                                    ready_player_ids: available.ready_player_ids,
                                                    pending_player_ids: available.pending_player_ids,
                                                }
                                            }
                                        };
                                        shared_state.update_play_again_status(play_again_status);
                                    }
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

    let _ = tx.send(ClientMessage {
        message: Some(client_message::Message::Disconnect(DisconnectRequest {})),
    }).await;

    break;
    }

    Ok(())
}
