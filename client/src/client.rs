use ringbuffer::{AllocRingBuffer, RingBuffer};
use common::snake_game_service_client::SnakeGameServiceClient;
use common::{ClientMessage, client_message, ConnectRequest, DisconnectRequest, ListLobbiesRequest, CreateLobbyRequest, JoinLobbyRequest, LeaveLobbyRequest, MarkReadyRequest, StartGameRequest, PlayAgainRequest, AddBotRequest, KickFromLobbyRequest, LobbySettings, log, TurnCommand};
use tokio::sync::mpsc;
use crate::state::{MenuCommand, GameCommand, ClientCommand, SharedState, AppState, PlayAgainStatus};
use crate::config::{ConfigManager, FileContentConfigProvider, Config, YamlConfigSerializer};
use crate::constants::CHAT_BUFFER_SIZE;

fn new_client_message(message: client_message::Message) -> ClientMessage {
    ClientMessage {
        version: common::version::get_version().to_string(),
        message: Some(message),
    }
}

#[derive(Clone)]
pub struct GrpcLoggingSender<T> {
    inner: mpsc::Sender<T>,
}

impl GrpcLoggingSender<ClientMessage> {
    pub fn new(inner: mpsc::Sender<ClientMessage>) -> Self {
        Self { inner }
    }

    pub async fn send(&self, value: ClientMessage) -> Result<(), mpsc::error::SendError<ClientMessage>> {
        let is_ping = matches!(&value.message, Some(client_message::Message::Ping(_)));
        if !is_ping {
            log!("Sending: {:?}", value);
        }
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

    tx.send(new_client_message(client_message::Message::Connect(ConnectRequest {
        client_id: client_id.clone(),
    })))
    .await?;

    tx.send(new_client_message(client_message::Message::ListLobbies(ListLobbiesRequest {})))
    .await?;

    let mut ping_interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
    let mut ping_counter: u64 = 0;
    let mut last_ping_id: Option<u64> = None;

    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                ping_counter += 1;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                let ping_msg = new_client_message(client_message::Message::Ping(common::PingRequest {
                    ping_id: ping_counter,
                    client_timestamp_ms: now,
                }));
                if tx.send(ping_msg).await.is_ok() {
                    last_ping_id = Some(ping_counter);
                } else {
                    break;
                }
            }

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
                                        max_food_count: config.max_food_count,
                                        food_spawn_probability: config.food_spawn_probability,
                                        dead_snake_behavior: config.dead_snake_behavior.into(),
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
                            MenuCommand::AddBot { bot_type } => {
                                Some(client_message::Message::AddBot(AddBotRequest {
                                    bot_type: bot_type as i32,
                                }))
                            }
                            MenuCommand::KickFromLobby { player_id } => {
                                Some(client_message::Message::KickFromLobby(KickFromLobbyRequest {
                                    player_id,
                                }))
                            }
                            MenuCommand::Disconnect => {
                                Some(client_message::Message::Disconnect(DisconnectRequest {}))
                            }
                            MenuCommand::InLobbyChatMessage { message } => {
                                Some(client_message::Message::InLobbyChat(common::InLobbyChatMessageRequest {
                                    message,
                                }))
                            },
                            MenuCommand::LobbyListChatMessage { message } => {
                                Some(client_message::Message::LobbyListChat(common::LobbyListChatMessageRequest {
                                    message,
                                }))
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
                    if tx.send(new_client_message(msg)).await.is_err() {
                        break;
                    }
                }
            }

            result = response_stream.message() => {
                match result {
                    Ok(Some(server_msg)) => {
                        let is_pong = matches!(&server_msg.message, Some(common::server_message::Message::Pong(_)));
                        if !is_pong {
                            log!("Received: {:?}", server_msg);
                        }

                        if let Some(msg) = server_msg.message {
                            match msg {
                                common::server_message::Message::LobbyList(lobby_list) => {
                                    let chat_messages = match shared_state.get_state() {
                                        AppState::LobbyList { chat_messages, .. } => chat_messages,
                                        _ => AllocRingBuffer::new(CHAT_BUFFER_SIZE),
                                    };
                                    shared_state.set_state(AppState::LobbyList {
                                        lobbies: lobby_list.lobbies,
                                        chat_messages,
                                    });
                                }
                                common::server_message::Message::LobbyUpdate(update) => {
                                    if let Some(lobby) = update.lobby {
                                        match shared_state.get_state() {
                                            AppState::InLobby { event_log, .. } => {
                                                shared_state.set_state(AppState::InLobby {
                                                    details: lobby,
                                                    event_log,
                                                });
                                            }
                                            AppState::LobbyList { .. } => {
                                                shared_state.set_state(AppState::InLobby {
                                                    details: lobby,
                                                    event_log: AllocRingBuffer::new(CHAT_BUFFER_SIZE),
                                                });
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                common::server_message::Message::PlayerJoined(notification) => {
                                    if let Some(identity) = &notification.identity {
                                        if identity.is_bot {
                                            let bot_type_name = match common::BotType::try_from(identity.bot_type).unwrap_or(common::BotType::Unspecified) {
                                                common::BotType::Efficient => "Efficient",
                                                common::BotType::Random => "Random",
                                                _ => "Unknown",
                                            };

                                            let host_name = match shared_state.get_state() {
                                                AppState::InLobby { details, .. } => {
                                                    details.creator.as_ref()
                                                        .map(|c| c.player_id.clone())
                                                        .unwrap_or_else(|| "Host".to_string())
                                                }
                                                _ => "Host".to_string()
                                            };

                                            shared_state.add_event_log(format!("{} added {} (Bot - {})", host_name, identity.player_id, bot_type_name));
                                        } else {
                                            shared_state.add_event_log(format!("{} joined", identity.player_id));
                                        }
                                    }
                                }
                                common::server_message::Message::PlayerLeft(notification) => {
                                    if let Some(identity) = &notification.identity {
                                        if identity.is_bot {
                                            let bot_type_name = match common::BotType::try_from(identity.bot_type).unwrap_or(common::BotType::Unspecified) {
                                                common::BotType::Efficient => "Efficient",
                                                common::BotType::Random => "Random",
                                                _ => "Unknown",
                                            };

                                            let host_name = match shared_state.get_state() {
                                                AppState::InLobby { details, .. } => {
                                                    details.creator.as_ref()
                                                        .map(|c| c.player_id.clone())
                                                        .unwrap_or_else(|| "Host".to_string())
                                                }
                                                _ => "Host".to_string()
                                            };

                                            shared_state.add_event_log(format!("{} removed {} (Bot - {})", host_name, identity.player_id, bot_type_name));
                                        } else {
                                            shared_state.add_event_log(format!("{} left", identity.player_id));
                                        }
                                    }
                                }
                                common::server_message::Message::PlayerReady(notification) => {
                                    if let Some(identity) = &notification.identity {
                                        if !identity.is_bot {
                                            let status = if notification.ready { "ready" } else { "not ready" };
                                            shared_state.add_event_log(format!("{} is {}", identity.player_id, status));
                                        }
                                    }
                                }
                                common::server_message::Message::Error(err) => {
                                    shared_state.set_error(err.message);
                                }
                                common::server_message::Message::LobbyListUpdate(_) => {
                                    if tx.send(new_client_message(
                                        client_message::Message::ListLobbies(ListLobbiesRequest {})
                                    )).await.is_err() {
                                        break;
                                    }
                                }
                                common::server_message::Message::ServerShuttingDown(_) => {
                                    shared_state.set_error("Server is shutting down".to_string());
                                    shared_state.set_should_close();
                                    break;
                                }
                                common::server_message::Message::LobbyClosed(notification) => {
                                    if matches!(shared_state.get_state(), AppState::InLobby { .. }) {
                                        shared_state.set_error(notification.message);
                                        if tx.send(new_client_message(
                                            client_message::Message::ListLobbies(ListLobbiesRequest {})
                                        )).await.is_err() {
                                            break;
                                        }
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
                                    let winner_name = game_over.winner.as_ref()
                                        .map(|w| w.player_id.clone())
                                        .unwrap_or_else(|| "None".to_string());
                                    log!("Game over! Winner: {}", winner_name);

                                    let last_game_state = match shared_state.get_state() {
                                        AppState::InGame { game_state, .. } => game_state,
                                        _ => None,
                                    };

                                    let reason = common::GameEndReason::try_from(game_over.reason)
                                        .unwrap_or(common::GameEndReason::Unspecified);

                                    shared_state.set_state(AppState::GameOver {
                                        scores: game_over.scores,
                                        winner: game_over.winner,
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
                                                    ready_players: available.ready_players,
                                                    pending_players: available.pending_players,
                                                }
                                            }
                                        };
                                        shared_state.update_play_again_status(play_again_status);
                                    }
                                }
                                common::server_message::Message::Pong(pong) => {
                                    if last_ping_id == Some(pong.ping_id) {
                                        let now = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap()
                                            .as_millis() as u64;
                                        let rtt = now.saturating_sub(pong.client_timestamp_ms);
                                        shared_state.set_ping(rtt);
                                    }
                                },
                                common::server_message::Message::LobbyListChat(notification) => {
                                    if let Some(sender) = &notification.sender {
                                        let mut state = shared_state.get_state_mut();

                                        let you_message = if sender.player_id == client_id {
                                            " (You)".to_string()
                                        } else {
                                            "".to_string()
                                        };

                                        if let AppState::LobbyList { chat_messages, .. } = &mut *state {
                                            chat_messages.enqueue(format!("{}{}: {}", sender.player_id, you_message, notification.message));
                                        }
                                    }
                                },
                                common::server_message::Message::InLobbyChat(notification) => {
                                    if let Some(sender) = &notification.sender {

                                        let mut state = shared_state.get_state_mut();

                                        let you_message = if sender.player_id == client_id {
                                            " (You)".to_string()
                                        } else {
                                            "".to_string()
                                        };

                                        if let AppState::InLobby { event_log, .. } = &mut *state {
                                            event_log.enqueue(format!("{}{}: {}", sender.player_id, you_message, notification.message));
                                        }
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

    let _ = tx.send(new_client_message(
        client_message::Message::Disconnect(DisconnectRequest {})
    )).await;

    break;
    }

    Ok(())
}
