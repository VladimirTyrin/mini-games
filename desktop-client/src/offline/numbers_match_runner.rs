use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use common::games::{GameBroadcaster, GameResolver, GameSession, GameSessionConfig};
use common::games::numbers_match::{NumbersMatchSession, NumbersMatchSessionState};
use common::identifiers::{ClientId, PlayerId};
use common::lobby::Lobby;
use common::proto::numbers_match::HintMode;
use common::replay::{ReplayRecorder, save_replay, generate_replay_filename};
use common::{ReplayGame, InGameCommand, in_game_command, PlayerIdentity, NumbersMatchLobbySettings};
use common::version::VERSION;
use crate::config::{NumbersMatchLobbyConfig, ReplayConfig};
use crate::state::{ClientCommand, GameCommand, MenuCommand, SharedState, NumbersMatchGameCommand};

use super::LocalBroadcaster;

pub async fn run_numbers_match_game(
    shared_state: &SharedState,
    command_rx: &mut mpsc::UnboundedReceiver<ClientCommand>,
    player_id: &PlayerId,
    lobby: &Lobby,
    cfg: &NumbersMatchLobbyConfig,
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

    let nm_settings = NumbersMatchLobbySettings {
        hint_mode: cfg.hint_mode.into(),
    };

    let players: Vec<PlayerIdentity> = game_config.human_players.iter()
        .map(|p| PlayerIdentity { player_id: p.to_string(), is_bot: false })
        .collect();

    let replay_recorder = Arc::new(Mutex::new(ReplayRecorder::new(
        VERSION.to_string(),
        ReplayGame::NumbersMatch,
        seed,
        Some(common::lobby_settings::Settings::NumbersMatch(nm_settings)),
        players,
    )));

    let hint_mode = match cfg.hint_mode {
        HintMode::Limited => common::games::numbers_match::HintMode::Limited,
        HintMode::Unlimited => common::games::numbers_match::HintMode::Unlimited,
        HintMode::Disabled => common::games::numbers_match::HintMode::Disabled,
        HintMode::Unspecified => common::games::numbers_match::HintMode::Limited,
    };

    let session_state = match NumbersMatchSessionState::create(&game_config, hint_mode, seed, Some(replay_recorder.clone())) {
        Ok(s) => s,
        Err(_) => return,
    };

    let session_for_commands = GameSession::NumbersMatch(session_state.clone());
    let client_id = ClientId::new(player_id_str.clone());

    let replay_recorder_for_save = replay_recorder.clone();
    let save_replays = replay_config.save;
    let replay_location = replay_config.location.clone();
    let shared_state_for_path = shared_state.clone();

    let mut game_handle = tokio::spawn(async move {
        NumbersMatchSession::run(&game_config, &session_state, &broadcaster).await
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
                    let file_name = generate_replay_filename(ReplayGame::NumbersMatch, VERSION);
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
                    ClientCommand::Game(GameCommand::NumbersMatch(nm_cmd)) => {
                        let in_game_command = match nm_cmd {
                            NumbersMatchGameCommand::RemovePair { first_index, second_index } => {
                                InGameCommand {
                                    command: Some(in_game_command::Command::NumbersMatch(
                                        common::NumbersMatchInGameCommand {
                                            command: Some(common::proto::numbers_match::numbers_match_in_game_command::Command::RemovePair(
                                                common::proto::numbers_match::RemovePairCommand { first_index, second_index }
                                            )),
                                        }
                                    )),
                                }
                            }
                            NumbersMatchGameCommand::Refill => {
                                InGameCommand {
                                    command: Some(in_game_command::Command::NumbersMatch(
                                        common::NumbersMatchInGameCommand {
                                            command: Some(common::proto::numbers_match::numbers_match_in_game_command::Command::Refill(
                                                common::proto::numbers_match::RefillCommand {}
                                            )),
                                        }
                                    )),
                                }
                            }
                            NumbersMatchGameCommand::RequestHint => {
                                InGameCommand {
                                    command: Some(in_game_command::Command::NumbersMatch(
                                        common::NumbersMatchInGameCommand {
                                            command: Some(common::proto::numbers_match::numbers_match_in_game_command::Command::RequestHint(
                                                common::proto::numbers_match::RequestHintCommand {}
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
