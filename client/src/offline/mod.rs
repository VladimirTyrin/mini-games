mod broadcaster;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use common::games::{GameBroadcaster, GameResolver, GameSession, GameSessionConfig};
use common::games::snake::{
    SnakeSessionSettings, SnakeSessionState, SnakeSession,
    WallCollisionMode as SnakeWallCollisionMode, DeadSnakeBehavior as SnakeDeadSnakeBehavior,
};
use common::games::tictactoe::{
    TicTacToeSessionSettings, TicTacToeSessionState, TicTacToeSession, FirstPlayerMode,
};
use common::identifiers::{BotId, ClientId, LobbyId, PlayerId};
use common::lobby::{Lobby, LobbySettings, BotType as LobbyBotType};
use common::replay::{ReplayRecorder, save_replay, generate_replay_filename};
use common::{ReplayGame, InGameCommand, in_game_command, PlayerIdentity, SnakeLobbySettings, TicTacToeLobbySettings};
use common::version::VERSION;
use crate::config::{SnakeLobbyConfig, TicTacToeLobbyConfig, ReplayConfig, get_config_manager};
use crate::state::{
    AppState, ClientCommand, GameCommand, LobbyConfig, MenuCommand,
    SharedState, SnakeGameCommand, TicTacToeGameCommand,
};
use crate::constants::CHAT_BUFFER_SIZE;
use ringbuffer::AllocRingBuffer;

pub use broadcaster::LocalBroadcaster;

pub async fn local_game_task(
    client_id: String,
    shared_state: SharedState,
    mut command_rx: mpsc::UnboundedReceiver<ClientCommand>,
) {
    let player_id = PlayerId::new(client_id);
    let mut lobby: Option<Lobby> = None;
    let mut config: Option<LobbyConfig> = None;

    loop {
        let Some(command) = command_rx.recv().await else {
            break;
        };

        match command {
            ClientCommand::Menu(MenuCommand::CreateLobby { name, config: lobby_config }) => {
                let (created_lobby, created_config) = create_lobby(&player_id, name, lobby_config);
                let details = created_lobby.to_details();
                shared_state.set_state(AppState::InLobby {
                    details,
                    event_log: AllocRingBuffer::new(CHAT_BUFFER_SIZE),
                });
                lobby = Some(created_lobby);
                config = Some(created_config);
            }

            ClientCommand::Menu(MenuCommand::LeaveLobby) => {
                lobby = None;
                config = None;
                shared_state.set_state(AppState::LobbyList {
                    lobbies: vec![],
                    chat_messages: AllocRingBuffer::new(CHAT_BUFFER_SIZE),
                });
            }

            ClientCommand::Menu(MenuCommand::AddBot { bot_type }) => {
                if let Some(ref mut l) = lobby {
                    let lobby_bot_type = match bot_type {
                        crate::state::BotType::Snake(t) => LobbyBotType::Snake(t),
                        crate::state::BotType::TicTacToe(t) => LobbyBotType::TicTacToe(t),
                    };
                    if l.add_bot(lobby_bot_type).is_some() {
                        update_lobby_ui(&shared_state, l);
                    }
                }
            }

            ClientCommand::Menu(MenuCommand::KickFromLobby { player_id: kick_id }) => {
                if let Some(ref mut l) = lobby {
                    l.remove_bot(&BotId::new(kick_id));
                    update_lobby_ui(&shared_state, l);
                }
            }

            ClientCommand::Menu(MenuCommand::MarkReady { ready }) => {
                if let Some(ref mut l) = lobby {
                    l.set_ready(&player_id, ready);
                    update_lobby_ui(&shared_state, l);
                }
            }

            ClientCommand::Menu(MenuCommand::BecomeObserver) => {
                if let Some(ref mut l) = lobby {
                    if l.player_to_observer(&player_id) {
                        update_lobby_ui(&shared_state, l);
                    }
                }
            }

            ClientCommand::Menu(MenuCommand::BecomePlayer) => {
                if let Some(ref mut l) = lobby {
                    if l.observer_to_player(&player_id) {
                        update_lobby_ui(&shared_state, l);
                    }
                }
            }

            ClientCommand::Menu(MenuCommand::StartGame) | ClientCommand::Menu(MenuCommand::PlayAgain) => {
                if let (Some(l), Some(c)) = (lobby.take(), config.take()) {
                    run_game(&shared_state, &mut command_rx, &player_id, l.clone(), c.clone()).await;
                    lobby = Some(l);
                    config = Some(c);
                }
            }

            _ => {}
        }
    }
}

fn create_lobby(
    player_id: &PlayerId,
    name: String,
    lobby_config: crate::state::LobbyConfig,
) -> (Lobby, LobbyConfig) {
    let (settings, max_players, offline_config) = match lobby_config {
        crate::state::LobbyConfig::Snake(cfg) => {
            let snake_settings = common::SnakeLobbySettings {
                field_width: cfg.field_width,
                field_height: cfg.field_height,
                wall_collision_mode: cfg.wall_collision_mode.into(),
                dead_snake_behavior: cfg.dead_snake_behavior.into(),
                tick_interval_ms: cfg.tick_interval_ms,
                max_food_count: cfg.max_food_count,
                food_spawn_probability: cfg.food_spawn_probability,
            };
            (LobbySettings::Snake(snake_settings), cfg.max_players, LobbyConfig::Snake(cfg))
        }
        crate::state::LobbyConfig::TicTacToe(cfg) => {
            let ttt_settings = common::TicTacToeLobbySettings {
                field_width: cfg.field_width,
                field_height: cfg.field_height,
                win_count: cfg.win_count,
                first_player: common::FirstPlayerMode::Random.into(),
            };
            (LobbySettings::TicTacToe(ttt_settings), 2, LobbyConfig::TicTacToe(cfg))
        }
    };

    let creator_client_id = ClientId::new(player_id.to_string());
    let mut lobby = Lobby::new(
        LobbyId::new("offline".to_string()),
        name,
        creator_client_id,
        max_players,
        settings,
    );
    lobby.add_player(player_id.clone());
    lobby.set_ready(player_id, true);

    (lobby, offline_config)
}

fn update_lobby_ui(shared_state: &SharedState, lobby: &Lobby) {
    shared_state.set_state(AppState::InLobby {
        details: lobby.to_details(),
        event_log: AllocRingBuffer::new(CHAT_BUFFER_SIZE),
    });
}

async fn run_game(
    shared_state: &SharedState,
    command_rx: &mut mpsc::UnboundedReceiver<ClientCommand>,
    player_id: &PlayerId,
    lobby: Lobby,
    config: LobbyConfig,
) {
    let session_id = format!("offline_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0));

    let is_observer = lobby.observers.contains(player_id);

    shared_state.set_state(AppState::InGame {
        session_id: session_id.clone(),
        game_state: None,
        is_observer,
    });

    let replay_config = get_config_manager()
        .get_config()
        .expect("Failed to load config")
        .replays;

    match config {
        LobbyConfig::Snake(ref cfg) => {
            run_snake_game(shared_state, command_rx, player_id, &lobby, cfg, &replay_config).await;
        }
        LobbyConfig::TicTacToe(ref cfg) => {
            run_tictactoe_game(shared_state, command_rx, player_id, &lobby, cfg, &replay_config).await;
        }
    }
}

async fn run_snake_game(
    shared_state: &SharedState,
    command_rx: &mut mpsc::UnboundedReceiver<ClientCommand>,
    player_id: &PlayerId,
    lobby: &Lobby,
    cfg: &SnakeLobbyConfig,
    replay_config: &ReplayConfig,
) {
    let settings = SnakeSessionSettings {
        field_width: cfg.field_width as usize,
        field_height: cfg.field_height as usize,
        wall_collision_mode: config_wall_mode_to_engine(cfg.wall_collision_mode),
        dead_snake_behavior: config_dead_snake_to_engine(cfg.dead_snake_behavior),
        max_food_count: cfg.max_food_count as usize,
        food_spawn_probability: cfg.food_spawn_probability,
        tick_interval: Duration::from_millis(cfg.tick_interval_ms as u64),
    };

    let bots: HashMap<BotId, LobbyBotType> = lobby.bots.iter()
        .map(|(id, bt)| (id.clone(), *bt))
        .collect();

    let game_config = GameSessionConfig {
        session_id: "offline".to_string(),
        human_players: lobby.players.keys().cloned().collect(),
        observers: lobby.observers.clone(),
        bots,
    };

    let player_id_str = player_id.to_string();
    let broadcaster = LocalBroadcaster::new(shared_state.clone(), player_id_str.clone());

    let seed: u64 = rand::random();

    let snake_settings = SnakeLobbySettings {
        field_width: cfg.field_width,
        field_height: cfg.field_height,
        wall_collision_mode: cfg.wall_collision_mode.into(),
        dead_snake_behavior: cfg.dead_snake_behavior.into(),
        tick_interval_ms: cfg.tick_interval_ms,
        max_food_count: cfg.max_food_count,
        food_spawn_probability: cfg.food_spawn_probability,
    };

    let players: Vec<PlayerIdentity> = game_config.human_players.iter()
        .map(|p| PlayerIdentity { player_id: p.to_string(), is_bot: false })
        .chain(game_config.bots.keys().map(|b| PlayerIdentity { player_id: b.to_player_id().to_string(), is_bot: true }))
        .collect();

    let replay_recorder = Arc::new(Mutex::new(ReplayRecorder::new(
        VERSION.to_string(),
        ReplayGame::Snake,
        seed,
        Some(common::lobby_settings::Settings::Snake(snake_settings)),
        players,
    )));

    let session_state = SnakeSessionState::create(&game_config, &settings, seed, Some(replay_recorder.clone()));
    let session_for_commands = GameSession::Snake(session_state.clone());
    let client_id = ClientId::new(player_id_str.clone());

    let replay_recorder_for_save = replay_recorder.clone();
    let save_replays = replay_config.save;
    let replay_location = replay_config.location.clone();
    let shared_state_for_path = shared_state.clone();

    let mut game_handle = tokio::spawn(async move {
        SnakeSession::run(game_config, session_state, broadcaster).await
    });

    loop {
        tokio::select! {
            result = &mut game_handle => {
                if let Ok(notification) = result {
                    let broadcaster = LocalBroadcaster::new(shared_state.clone(), player_id_str.clone());
                    broadcaster.broadcast_game_over(notification, vec![]).await;
                }

                if save_replays {
                    let mut recorder = replay_recorder_for_save.lock().await;
                    let replay = recorder.finalize();
                    let file_name = generate_replay_filename(ReplayGame::Snake, VERSION);
                    let replay_dir = std::path::Path::new(&replay_location);
                    if let Err(e) = std::fs::create_dir_all(replay_dir) {
                        common::log!("Failed to create replay directory: {}", e);
                    } else {
                        let file_path = replay_dir.join(&file_name);
                        match save_replay(&file_path, &replay) {
                            Ok(_) => {
                                common::log!("Replay saved to: {}", file_path.display());
                                shared_state_for_path.set_last_replay_path(Some(file_path));
                            }
                            Err(e) => {
                                common::log!("Failed to save replay: {}", e);
                            }
                        }
                    }
                }
                break;
            }
            Some(command) = command_rx.recv() => {
                match command {
                    ClientCommand::Game(GameCommand::Snake(SnakeGameCommand::SendTurn { direction })) => {
                        let in_game_command = InGameCommand {
                            command: Some(in_game_command::Command::Snake(
                                common::SnakeInGameCommand {
                                    command: Some(common::proto::snake::snake_in_game_command::Command::Turn(
                                        common::TurnCommand { direction: direction as i32 }
                                    )),
                                }
                            )),
                        };
                        GameResolver::handle_command(&session_for_commands, &client_id, in_game_command).await;
                    }
                    ClientCommand::Menu(MenuCommand::LeaveLobby) => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn run_tictactoe_game(
    shared_state: &SharedState,
    command_rx: &mut mpsc::UnboundedReceiver<ClientCommand>,
    player_id: &PlayerId,
    lobby: &Lobby,
    cfg: &TicTacToeLobbyConfig,
    replay_config: &ReplayConfig,
) {
    let settings = TicTacToeSessionSettings {
        field_width: cfg.field_width as usize,
        field_height: cfg.field_height as usize,
        win_count: cfg.win_count as usize,
        first_player_mode: FirstPlayerMode::Random,
    };

    let bots: HashMap<BotId, LobbyBotType> = lobby.bots.iter()
        .map(|(id, bt)| (id.clone(), *bt))
        .collect();

    let total_players = lobby.players.len() + bots.len();
    if total_players != 2 {
        return;
    }

    let game_config = GameSessionConfig {
        session_id: "offline".to_string(),
        human_players: lobby.players.keys().cloned().collect(),
        observers: lobby.observers.clone(),
        bots,
    };

    let player_id_str = player_id.to_string();
    let broadcaster = LocalBroadcaster::new(shared_state.clone(), player_id_str.clone());

    let seed: u64 = rand::random();

    let ttt_settings = TicTacToeLobbySettings {
        field_width: cfg.field_width,
        field_height: cfg.field_height,
        win_count: cfg.win_count,
        first_player: common::FirstPlayerMode::Random.into(),
    };

    let players: Vec<PlayerIdentity> = game_config.human_players.iter()
        .map(|p| PlayerIdentity { player_id: p.to_string(), is_bot: false })
        .chain(game_config.bots.keys().map(|b| PlayerIdentity { player_id: b.to_player_id().to_string(), is_bot: true }))
        .collect();

    let replay_recorder = Arc::new(Mutex::new(ReplayRecorder::new(
        VERSION.to_string(),
        ReplayGame::Tictactoe,
        seed,
        Some(common::lobby_settings::Settings::Tictactoe(ttt_settings)),
        players,
    )));

    let session_state = match TicTacToeSessionState::create(&game_config, &settings, seed, Some(replay_recorder.clone())) {
        Ok(s) => s,
        Err(_) => return,
    };

    let session_for_commands = GameSession::TicTacToe(session_state.clone());
    let client_id = ClientId::new(player_id_str.clone());

    let replay_recorder_for_save = replay_recorder.clone();
    let save_replays = replay_config.save;
    let replay_location = replay_config.location.clone();
    let shared_state_for_path = shared_state.clone();

    let mut game_handle = tokio::spawn(async move {
        TicTacToeSession::run(game_config, session_state, broadcaster).await
    });

    loop {
        tokio::select! {
            result = &mut game_handle => {
                if let Ok(notification) = result {
                    let broadcaster = LocalBroadcaster::new(shared_state.clone(), player_id_str.clone());
                    broadcaster.broadcast_game_over(notification, vec![]).await;
                }

                if save_replays {
                    let mut recorder = replay_recorder_for_save.lock().await;
                    let replay = recorder.finalize();
                    let file_name = generate_replay_filename(ReplayGame::Tictactoe, VERSION);
                    let replay_dir = std::path::Path::new(&replay_location);
                    if let Err(e) = std::fs::create_dir_all(replay_dir) {
                        common::log!("Failed to create replay directory: {}", e);
                    } else {
                        let file_path = replay_dir.join(&file_name);
                        match save_replay(&file_path, &replay) {
                            Ok(_) => {
                                common::log!("Replay saved to: {}", file_path.display());
                                shared_state_for_path.set_last_replay_path(Some(file_path));
                            }
                            Err(e) => {
                                common::log!("Failed to save replay: {}", e);
                            }
                        }
                    }
                }
                break;
            }
            Some(command) = command_rx.recv() => {
                match command {
                    ClientCommand::Game(GameCommand::TicTacToe(TicTacToeGameCommand::PlaceMark { x, y })) => {
                        let in_game_command = InGameCommand {
                            command: Some(in_game_command::Command::Tictactoe(
                                common::TicTacToeInGameCommand {
                                    command: Some(common::proto::tictactoe::tic_tac_toe_in_game_command::Command::Place(
                                        common::PlaceMarkCommand { x, y }
                                    )),
                                }
                            )),
                        };
                        GameResolver::handle_command(&session_for_commands, &client_id, in_game_command).await;
                    }
                    ClientCommand::Menu(MenuCommand::LeaveLobby) => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn config_wall_mode_to_engine(mode: common::WallCollisionMode) -> SnakeWallCollisionMode {
    match mode {
        common::WallCollisionMode::WrapAround => SnakeWallCollisionMode::WrapAround,
        common::WallCollisionMode::Death | common::WallCollisionMode::Unspecified => {
            SnakeWallCollisionMode::Death
        }
    }
}

fn config_dead_snake_to_engine(behavior: common::DeadSnakeBehavior) -> SnakeDeadSnakeBehavior {
    match behavior {
        common::DeadSnakeBehavior::StayOnField => SnakeDeadSnakeBehavior::StayOnField,
        common::DeadSnakeBehavior::Disappear | common::DeadSnakeBehavior::Unspecified => {
            SnakeDeadSnakeBehavior::Disappear
        }
    }
}
