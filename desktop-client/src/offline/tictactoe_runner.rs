use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use common::games::{GameBroadcaster, GameResolver, GameSession, GameSessionConfig};
use common::games::tictactoe::{
    TicTacToeSessionSettings, TicTacToeSessionState, TicTacToeSession, FirstPlayerMode,
};
use common::identifiers::{BotId, ClientId, PlayerId};
use common::lobby::{Lobby, BotType as LobbyBotType};
use common::replay::{ReplayRecorder, save_replay, generate_replay_filename};
use common::{ReplayGame, InGameCommand, in_game_command, PlayerIdentity, TicTacToeLobbySettings};
use common::version::VERSION;
use crate::config::{TicTacToeLobbyConfig, ReplayConfig};
use crate::state::{ClientCommand, GameCommand, MenuCommand, SharedState, TicTacToeGameCommand};

use super::LocalBroadcaster;

pub async fn run_tictactoe_game(
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
