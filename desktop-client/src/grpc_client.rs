use ringbuffer::{AllocRingBuffer, RingBuffer};
use common::proto::game_service::game_service_client::GameServiceClient;
use common::{ClientMessage, client_message, ConnectRequest, DisconnectRequest, ListLobbiesRequest, CreateLobbyRequest, JoinLobbyRequest, LeaveLobbyRequest, MarkReadyRequest, StartGameRequest, PlayAgainRequest, AddBotRequest, KickFromLobbyRequest, log, proto::snake::{SnakeLobbySettings, TurnCommand}, InGameCommand, in_game_command};
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
    initial_server_address: Option<String>,
    shared_state: SharedState,
    mut command_rx: mpsc::UnboundedReceiver<ClientCommand>,
    config_manager: ConfigManager<FileContentConfigProvider, Config, YamlConfigSerializer>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut server_address = match initial_server_address {
        Some(addr) => addr,
        None => {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                if let Some(new_address) = shared_state.take_retry_server_address() {
                    break new_address;
                }

                if shared_state.should_close() {
                    return Ok(());
                }
            }
        }
    };

    loop {
        let mut client = match GameServiceClient::connect(server_address.clone()).await {
            Ok(client) => {
                let mut config = config_manager.get_config().unwrap_or_default();
                config.server.address = Some(server_address.clone());
                if let Err(e) = config_manager.set_config(&config) {
                    log!("Failed to save server address to config: {}", e);
                }
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
                                let (max_players, settings) = match config {
                                    crate::state::LobbyConfig::Snake(snake_config) => {
                                        (snake_config.max_players, Some(common::LobbySettings {
                                            settings: Some(common::lobby_settings::Settings::Snake(SnakeLobbySettings {
                                                field_width: snake_config.field_width,
                                                field_height: snake_config.field_height,
                                                wall_collision_mode: snake_config.wall_collision_mode.into(),
                                                tick_interval_ms: snake_config.tick_interval_ms,
                                                max_food_count: snake_config.max_food_count,
                                                food_spawn_probability: snake_config.food_spawn_probability,
                                                dead_snake_behavior: snake_config.dead_snake_behavior.into(),
                                            })),
                                        }))
                                    }
                                    crate::state::LobbyConfig::TicTacToe(ttt_config) => {
                                        (2, Some(common::LobbySettings {
                                            settings: Some(common::lobby_settings::Settings::Tictactoe(common::proto::tictactoe::TicTacToeLobbySettings {
                                                field_width: ttt_config.field_width,
                                                field_height: ttt_config.field_height,
                                                win_count: ttt_config.win_count,
                                                first_player: 1,
                                            })),
                                        }))
                                    }
                                    crate::state::LobbyConfig::NumbersMatch(nm_config) => {
                                        (1, Some(common::LobbySettings {
                                            settings: Some(common::lobby_settings::Settings::NumbersMatch(common::proto::numbers_match::NumbersMatchLobbySettings {
                                                hint_mode: nm_config.hint_mode.into(),
                                            })),
                                        }))
                                    }
                                    crate::state::LobbyConfig::Puzzle2048(p_config) => {
                                        (1, Some(common::LobbySettings {
                                            settings: Some(common::lobby_settings::Settings::Puzzle2048(common::proto::puzzle2048::Puzzle2048LobbySettings {
                                                field_width: p_config.field_width,
                                                field_height: p_config.field_height,
                                                target_value: p_config.target_value,
                                            })),
                                        }))
                                    }
                                };

                                Some(client_message::Message::CreateLobby(CreateLobbyRequest {
                                    lobby_name: name,
                                    max_players,
                                    settings,
                                }))
                            }
                            MenuCommand::JoinLobby { lobby_id, join_as_observer } => {
                                Some(client_message::Message::JoinLobby(JoinLobbyRequest {
                                    lobby_id,
                                    join_as_observer,
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
                                let proto_bot_type = match bot_type {
                                    crate::state::BotType::Snake(snake_bot) => {
                                        Some(common::add_bot_request::BotType::SnakeBot(snake_bot as i32))
                                    }
                                    crate::state::BotType::TicTacToe(ttt_bot) => {
                                        Some(common::add_bot_request::BotType::TictactoeBot(ttt_bot as i32))
                                    }
                                };
                                Some(client_message::Message::AddBot(AddBotRequest {
                                    bot_type: proto_bot_type,
                                }))
                            }
                            MenuCommand::KickFromLobby { player_id } => {
                                Some(client_message::Message::KickFromLobby(KickFromLobbyRequest {
                                    player_id,
                                }))
                            }
                            MenuCommand::BecomeObserver => {
                                Some(client_message::Message::BecomeObserver(common::BecomeObserverFromPlayerRequest {}))
                            }
                            MenuCommand::BecomePlayer => {
                                Some(client_message::Message::BecomePlayer(common::BecomePlayerFromObserverRequest {}))
                            }
                            MenuCommand::MakePlayerObserver { player_id } => {
                                Some(client_message::Message::MakeObserver(common::MakePlayerObserverRequest {
                                    player_id,
                                }))
                            }
                            MenuCommand::Disconnect => {
                                Some(client_message::Message::Disconnect(DisconnectRequest {}))
                            }
                            MenuCommand::InLobbyChatMessage { message } => {
                                Some(client_message::Message::InLobbyChat(common::InLobbyChatMessage {
                                    message,
                                }))
                            },
                            MenuCommand::LobbyListChatMessage { message } => {
                                Some(client_message::Message::LobbyListChat(common::LobbyListChatMessage {
                                    message,
                                }))
                            }
                        }
                    }
                    ClientCommand::Game(game_cmd) => {
                        match game_cmd {
                            GameCommand::Snake(snake_cmd) => {
                                match snake_cmd {
                                    crate::state::SnakeGameCommand::SendTurn { direction } => {
                                        Some(client_message::Message::InGame(InGameCommand {
                                            command: Some(in_game_command::Command::Snake(
                                                common::proto::snake::SnakeInGameCommand {
                                                    command: Some(common::proto::snake::snake_in_game_command::Command::Turn(
                                                        TurnCommand {
                                                            direction: direction as i32,
                                                        }
                                                    ))
                                                }
                                            ))
                                        }))
                                    }
                                }
                            }
                            GameCommand::TicTacToe(ttt_cmd) => {
                                match ttt_cmd {
                                    crate::state::TicTacToeGameCommand::PlaceMark { x, y } => {
                                        Some(client_message::Message::InGame(InGameCommand {
                                            command: Some(in_game_command::Command::Tictactoe(
                                                common::proto::tictactoe::TicTacToeInGameCommand {
                                                    command: Some(common::proto::tictactoe::tic_tac_toe_in_game_command::Command::Place(
                                                        common::proto::tictactoe::PlaceMarkCommand {
                                                            x,
                                                            y,
                                                        }
                                                    ))
                                                }
                                            ))
                                        }))
                                    }
                                }
                            }
                            GameCommand::NumbersMatch(nm_cmd) => {
                                use crate::state::NumbersMatchGameCommand;
                                match nm_cmd {
                                    NumbersMatchGameCommand::RemovePair { first_index, second_index } => {
                                        Some(client_message::Message::InGame(InGameCommand {
                                            command: Some(in_game_command::Command::NumbersMatch(
                                                common::NumbersMatchInGameCommand {
                                                    command: Some(common::proto::numbers_match::numbers_match_in_game_command::Command::RemovePair(
                                                        common::proto::numbers_match::RemovePairCommand { first_index, second_index }
                                                    ))
                                                }
                                            ))
                                        }))
                                    }
                                    NumbersMatchGameCommand::Refill => {
                                        Some(client_message::Message::InGame(InGameCommand {
                                            command: Some(in_game_command::Command::NumbersMatch(
                                                common::NumbersMatchInGameCommand {
                                                    command: Some(common::proto::numbers_match::numbers_match_in_game_command::Command::Refill(
                                                        common::proto::numbers_match::RefillCommand {}
                                                    ))
                                                }
                                            ))
                                        }))
                                    }
                                    NumbersMatchGameCommand::RequestHint => {
                                        Some(client_message::Message::InGame(InGameCommand {
                                            command: Some(in_game_command::Command::NumbersMatch(
                                                common::NumbersMatchInGameCommand {
                                                    command: Some(common::proto::numbers_match::numbers_match_in_game_command::Command::RequestHint(
                                                        common::proto::numbers_match::RequestHintCommand {}
                                                    ))
                                                }
                                            ))
                                        }))
                                    }
                                }
                            }
                            GameCommand::Puzzle2048(p_cmd) => {
                                use crate::state::Puzzle2048GameCommand;
                                match p_cmd {
                                    Puzzle2048GameCommand::Move { direction } => {
                                        Some(client_message::Message::InGame(InGameCommand {
                                            command: Some(in_game_command::Command::Puzzle2048(
                                                common::Puzzle2048InGameCommand {
                                                    command: Some(common::proto::puzzle2048::puzzle2048_in_game_command::Command::Move(
                                                        common::proto::puzzle2048::MoveCommand {
                                                            direction: direction as i32,
                                                        }
                                                    ))
                                                }
                                            ))
                                        }))
                                    }
                                }
                            }
                        }
                    }
                };

                if let Some(msg) = message
                    && tx.send(new_client_message(msg)).await.is_err() {
                        break;
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
                                    if let Some(lobby) = update.details {
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
                                    if let Some(identity) = &notification.player {
                                        if identity.is_bot {
                                            let host_name = match shared_state.get_state() {
                                                AppState::InLobby { details, .. } => {
                                                    details.creator.as_ref()
                                                        .map(|c| c.player_id.clone())
                                                        .unwrap_or_else(|| "Host".to_string())
                                                }
                                                _ => "Host".to_string()
                                            };

                                            shared_state.add_event_log(format!("{} added {} (Bot)", host_name, identity.player_id));
                                        } else {
                                            shared_state.add_event_log(format!("{} joined", identity.player_id));
                                        }
                                    }
                                }
                                common::server_message::Message::PlayerLeft(notification) => {
                                    if let Some(identity) = &notification.player {
                                        if identity.is_bot {
                                            let host_name = match shared_state.get_state() {
                                                AppState::InLobby { details, .. } => {
                                                    details.creator.as_ref()
                                                        .map(|c| c.player_id.clone())
                                                        .unwrap_or_else(|| "Host".to_string())
                                                }
                                                _ => "Host".to_string()
                                            };

                                            shared_state.add_event_log(format!("{} removed {} (Bot)", host_name, identity.player_id));
                                        } else {
                                            shared_state.add_event_log(format!("{} left", identity.player_id));
                                        }
                                    }
                                }
                                common::server_message::Message::PlayerReady(notification) => {
                                    if let Some(identity) = &notification.player
                                        && !identity.is_bot {
                                            let status = if notification.ready { "ready" } else { "not ready" };
                                            shared_state.add_event_log(format!("{} is {}", identity.player_id, status));
                                        }
                                }
                                common::server_message::Message::Error(err) => {
                                    let is_version_mismatch = err.code == common::ErrorCode::VersionMismatch as i32;
                                    shared_state.set_error(err.message);
                                    if is_version_mismatch {
                                        shared_state.set_connection_mode(crate::state::ConnectionMode::TemporaryOffline);
                                        break;
                                    }
                                }
                                common::server_message::Message::LobbyListUpdate(_) => {
                                    if tx.send(new_client_message(
                                        client_message::Message::ListLobbies(ListLobbiesRequest {})
                                    )).await.is_err() {
                                        break;
                                    }
                                }
                                common::server_message::Message::Shutdown(_) => {
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
                                    let is_observer = match shared_state.get_state() {
                                        AppState::InLobby { details, .. } => {
                                            details.observers.iter().any(|o| o.player_id == client_id)
                                        }
                                        _ => false,
                                    };
                                    shared_state.set_state(AppState::InGame {
                                        session_id: notification.session_id.clone(),
                                        game_state: None,
                                        is_observer,
                                    });
                                }
                                common::server_message::Message::GameState(state) => {
                                    shared_state.update_game_state(state);
                                }
                                common::server_message::Message::GameOver(game_over) => {
                                    let winner_name = game_over.winner.as_ref()
                                        .map(|w| w.player_id.clone())
                                        .unwrap_or_else(|| "None".to_string());
                                    log!("Game over! Winner: {}", winner_name);

                                    let (last_game_state, is_observer) = match shared_state.get_state() {
                                        AppState::InGame { game_state, is_observer, .. } => (game_state, is_observer),
                                        _ => (None, false),
                                    };

                                    let game_info = match &game_over.game_info {
                                        Some(common::game_over_notification::GameInfo::SnakeInfo(info)) => {
                                            crate::state::GameEndInfo::Snake(*info)
                                        }
                                        Some(common::game_over_notification::GameInfo::TictactoeInfo(info)) => {
                                            crate::state::GameEndInfo::TicTacToe(*info)
                                        }
                                        Some(common::game_over_notification::GameInfo::NumbersMatchInfo(info)) => {
                                            crate::state::GameEndInfo::NumbersMatch(*info)
                                        }
                                        Some(common::game_over_notification::GameInfo::Puzzle2048Info(info)) => {
                                            crate::state::GameEndInfo::Puzzle2048(*info)
                                        }
                                        _ => crate::state::GameEndInfo::Snake(common::proto::snake::SnakeGameEndInfo {
                                            reason: common::proto::snake::SnakeGameEndReason::Unspecified as i32,
                                        }),
                                    };

                                    shared_state.set_state(AppState::GameOver {
                                        scores: game_over.scores,
                                        winner: game_over.winner,
                                        last_game_state,
                                        game_info,
                                        play_again_status: PlayAgainStatus::NotAvailable,
                                        is_observer,
                                    });
                                }
                                common::server_message::Message::PlayAgainStatus(notification) => {
                                    let play_again_status = if notification.available {
                                        PlayAgainStatus::Available {
                                            ready_players: notification.ready_players,
                                            pending_players: notification.pending_players,
                                        }
                                    } else {
                                        PlayAgainStatus::NotAvailable
                                    };
                                    shared_state.update_play_again_status(play_again_status);
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
                                        let mut inner = shared_state.get_state_mut();

                                        let you_message = if sender.player_id == client_id {
                                            " (You)".to_string()
                                        } else {
                                            "".to_string()
                                        };

                                        if let AppState::LobbyList { chat_messages, .. } = &mut inner.state {
                                            chat_messages.enqueue(format!("{}{}: {}", sender.player_id, you_message, notification.message));
                                        }
                                    }
                                },
                                common::server_message::Message::InLobbyChat(notification) => {
                                    if let Some(sender) = &notification.sender {

                                        let mut inner = shared_state.get_state_mut();

                                        let you_message = if sender.player_id == client_id {
                                            " (You)".to_string()
                                        } else {
                                            "".to_string()
                                        };

                                        if let AppState::InLobby { event_log, .. } = &mut inner.state {
                                            event_log.enqueue(format!("{}{}: {}", sender.player_id, you_message, notification.message));
                                        }
                                    }
                                }
                                common::server_message::Message::Connect(response) => {
                                    if !response.success {
                                        let error_msg = if response.error_message.is_empty() {
                                            "Connection rejected by server".to_string()
                                        } else {
                                            response.error_message.clone()
                                        };
                                        shared_state.set_error(error_msg);
                                        shared_state.set_connection_failed(true);
                                        break;
                                    }
                                }
                                common::server_message::Message::LobbyCreated(_) => {
                                }
                                common::server_message::Message::LobbyJoined(_) => {
                                }
                                common::server_message::Message::Kicked(notification) => {
                                    shared_state.set_error(format!("Kicked: {}", notification.reason));
                                    if tx.send(new_client_message(
                                        client_message::Message::ListLobbies(ListLobbiesRequest {})
                                    )).await.is_err() {
                                        break;
                                    }
                                }
                                common::server_message::Message::PlayerBecameObserver(notification) => {
                                    if let Some(player) = &notification.player {
                                        shared_state.add_event_log(format!("{} became an observer", player.player_id));
                                    }
                                }
                                common::server_message::Message::ObserverBecamePlayer(notification) => {
                                    if let Some(observer) = &notification.observer {
                                        shared_state.add_event_log(format!("{} became a player", observer.player_id));
                                    }
                                }
                                common::server_message::Message::ReplayFile(notification) => {
                                    log!("Received replay file: {} ({} bytes)",
                                         notification.suggested_file_name,
                                         notification.content.len());

                                    let config = config_manager.get_config().unwrap_or_default();
                                    if config.replays.save {
                                        let replay_dir = std::path::Path::new(&config.replays.location);
                                        if let Err(e) = std::fs::create_dir_all(replay_dir) {
                                            log!("Failed to create replay directory: {}", e);
                                        } else {
                                            let file_path = replay_dir.join(&notification.suggested_file_name);
                                            match std::fs::write(&file_path, &notification.content) {
                                                Ok(_) => {
                                                    log!("Replay saved to: {}", file_path.display());
                                                    shared_state.set_last_replay_path(Some(file_path));
                                                }
                                                Err(e) => {
                                                    log!("Failed to save replay: {}", e);
                                                }
                                            }
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

    if let Err(e) = tx.send(new_client_message(
        client_message::Message::Disconnect(DisconnectRequest {})
    )).await {
        log!("Failed to send disconnect request: {}", e);
    }

    break;
    }

    Ok(())
}
