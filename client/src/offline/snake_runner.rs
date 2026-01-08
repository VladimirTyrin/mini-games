use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use common::games::{GameBroadcaster, GameResolver, GameSession, GameSessionConfig};
use common::games::snake::{
    SnakeSessionSettings, SnakeSessionState, SnakeSession,
    WallCollisionMode as SnakeWallCollisionMode, DeadSnakeBehavior as SnakeDeadSnakeBehavior,
};
use common::identifiers::{BotId, ClientId, PlayerId};
use common::lobby::{Lobby, BotType as LobbyBotType};
use common::replay::{ReplayRecorder, save_replay, generate_replay_filename};
use common::{ReplayGame, InGameCommand, in_game_command, PlayerIdentity, SnakeLobbySettings, WallCollisionMode, DeadSnakeBehavior};
use common::version::VERSION;
use crate::config::{SnakeLobbyConfig, ReplayConfig};
use crate::state::{ClientCommand, GameCommand, MenuCommand, SharedState, SnakeGameCommand};

use super::LocalBroadcaster;

pub async fn run_snake_game(
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

fn config_wall_mode_to_engine(mode: WallCollisionMode) -> SnakeWallCollisionMode {
    match mode {
        WallCollisionMode::WrapAround => SnakeWallCollisionMode::WrapAround,
        WallCollisionMode::Death | WallCollisionMode::Unspecified => {
            SnakeWallCollisionMode::Death
        }
    }
}

fn config_dead_snake_to_engine(behavior: DeadSnakeBehavior) -> SnakeDeadSnakeBehavior {
    match behavior {
        DeadSnakeBehavior::StayOnField => SnakeDeadSnakeBehavior::StayOnField,
        DeadSnakeBehavior::Disappear | DeadSnakeBehavior::Unspecified => {
            SnakeDeadSnakeBehavior::Disappear
        }
    }
}
