use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use common::games::{GameBroadcaster, GameResolver, GameSession, GameSessionConfig};
use common::games::puzzle2048::{Puzzle2048Session, Puzzle2048SessionState};
use common::identifiers::{ClientId, PlayerId};
use common::lobby::Lobby;
use common::replay::{ReplayRecorder, save_replay, generate_replay_filename};
use common::{ReplayGame, InGameCommand, in_game_command, PlayerIdentity, Puzzle2048LobbySettings};
use common::version::VERSION;
use crate::config::{Puzzle2048LobbyConfig, ReplayConfig};
use crate::state::{ClientCommand, GameCommand, MenuCommand, SharedState, Puzzle2048GameCommand};

use super::LocalBroadcaster;

pub async fn run_puzzle2048_game(
    shared_state: &SharedState,
    command_rx: &mut mpsc::UnboundedReceiver<ClientCommand>,
    player_id: &PlayerId,
    lobby: &Lobby,
    cfg: &Puzzle2048LobbyConfig,
    replay_config: &ReplayConfig,
) {
    let game_config = GameSessionConfig {
        session_id: "offline".to_string(),
        human_players: lobby.players.keys().cloned().collect(),
        observers: lobby.observers.clone(),
        bots: std::collections::HashMap::new(),
    };

    let player_id_str = player_id.to_string();
    let broadcaster = LocalBroadcaster::new(shared_state.clone(), player_id_str.clone());

    let seed: u64 = rand::random();

    let p_settings = Puzzle2048LobbySettings {
        field_width: cfg.field_width,
        field_height: cfg.field_height,
        target_value: cfg.target_value,
    };

    let players: Vec<PlayerIdentity> = game_config.human_players.iter()
        .map(|p| PlayerIdentity { player_id: p.to_string(), is_bot: false })
        .collect();

    let replay_recorder = Arc::new(Mutex::new(ReplayRecorder::new(
        VERSION.to_string(),
        ReplayGame::Puzzle2048,
        seed,
        Some(common::lobby_settings::Settings::Puzzle2048(p_settings)),
        players,
    )));

    let session_state = match Puzzle2048SessionState::create(
        &game_config,
        cfg.field_width as usize,
        cfg.field_height as usize,
        cfg.target_value,
        seed,
        Some(replay_recorder.clone()),
    ) {
        Ok(s) => s,
        Err(_) => return,
    };

    let session_for_commands = GameSession::Puzzle2048(session_state.clone());
    let client_id = ClientId::new(player_id_str.clone());

    let replay_recorder_for_save = replay_recorder.clone();
    let save_replays = replay_config.save;
    let replay_location = replay_config.location.clone();
    let shared_state_for_path = shared_state.clone();

    let mut game_handle = tokio::spawn(async move {
        Puzzle2048Session::run(&game_config, &session_state, &broadcaster).await
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
                    let file_name = generate_replay_filename(ReplayGame::Puzzle2048, VERSION);
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
                    ClientCommand::Game(GameCommand::Puzzle2048(p_cmd)) => {
                        let in_game_command = match p_cmd {
                            Puzzle2048GameCommand::Move { direction } => {
                                InGameCommand {
                                    command: Some(in_game_command::Command::Puzzle2048(
                                        common::Puzzle2048InGameCommand {
                                            command: Some(common::proto::puzzle2048::puzzle2048_in_game_command::Command::Move(
                                                common::proto::puzzle2048::MoveCommand {
                                                    direction: direction as i32,
                                                }
                                            )),
                                        }
                                    )),
                                }
                            }
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
