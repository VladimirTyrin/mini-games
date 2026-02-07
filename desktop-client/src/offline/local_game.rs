use tokio::sync::mpsc;
use common::identifiers::{BotId, LobbyId, PlayerId};
use common::lobby::{Lobby, LobbySettings, BotType as LobbyBotType};
use crate::config::get_config_manager;
use crate::state::{
    AppState, ClientCommand, LobbyConfig, MenuCommand,
    SharedState,
};
use crate::constants::CHAT_BUFFER_SIZE;
use ringbuffer::AllocRingBuffer;

use super::numbers_match_runner::run_numbers_match_game;
use super::puzzle2048_runner::run_puzzle2048_game;
use super::snake_runner::run_snake_game;
use super::tictactoe_runner::run_tictactoe_game;

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
                if let Some(ref mut l) = lobby
                    && l.player_to_observer(&player_id)
                {
                    update_lobby_ui(&shared_state, l);
                }
            }

            ClientCommand::Menu(MenuCommand::BecomePlayer) => {
                if let Some(ref mut l) = lobby
                    && l.observer_to_player(&player_id)
                {
                    update_lobby_ui(&shared_state, l);
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
        crate::state::LobbyConfig::NumbersMatch(cfg) => {
            let nm_settings = common::NumbersMatchLobbySettings {
                hint_mode: cfg.hint_mode.into(),
            };
            (LobbySettings::NumbersMatch(nm_settings), 1, LobbyConfig::NumbersMatch(cfg))
        }
        crate::state::LobbyConfig::Puzzle2048(cfg) => {
            let p_settings = common::Puzzle2048LobbySettings {
                field_width: cfg.field_width,
                field_height: cfg.field_height,
                target_value: cfg.target_value,
            };
            (LobbySettings::Puzzle2048(p_settings), 1, LobbyConfig::Puzzle2048(cfg))
        }
    };

    let creator_client_id = common::identifiers::ClientId::new(player_id.to_string());
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
        LobbyConfig::NumbersMatch(ref cfg) => {
            run_numbers_match_game(shared_state, command_rx, player_id, &lobby, cfg, &replay_config).await;
        }
        LobbyConfig::Puzzle2048(ref cfg) => {
            run_puzzle2048_game(shared_state, command_rx, player_id, &lobby, cfg, &replay_config).await;
        }
    }
}
