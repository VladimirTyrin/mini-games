use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use common::{ClientId, LobbyId, PlayerId, BotId, log, ServerMessage, server_message};
use common::engine::snake::{GameState as SnakeGameState, Direction, DeathReason};
use common::engine::tictactoe::TicTacToeGameState;
use common::engine::session::GameSessionConfig;
use common::engine::session::snake_session::{
    SnakeSessionSettings,
    create_session as create_snake_session,
    run_game_loop as run_snake_game_loop,
};
use common::engine::session::tictactoe_session::{
    TicTacToeSessionSettings,
    create_session as create_tictactoe_session,
    run_game_loop as run_tictactoe_game_loop,
};
use common::replay::{ReplayRecorder, generate_replay_filename, save_replay_to_bytes, REPLAY_VERSION};
use common::{ReplayGame, InGameCommand, in_game_command, ReplayFileReadyNotification};
use crate::broadcaster::Broadcaster;
use crate::lobby_manager::{LobbyManager, LobbySettings};

pub type SessionId = String;

enum GameSession {
    Snake {
        game_state: Arc<Mutex<SnakeGameState>>,
        tick: Arc<Mutex<u64>>,
        replay_recorder: Arc<Mutex<ReplayRecorder>>,
    },
    TicTacToe {
        game_state: Arc<Mutex<TicTacToeGameState>>,
        turn_notify: Arc<Notify>,
        replay_recorder: Arc<Mutex<ReplayRecorder>>,
    },
}

pub struct GameSessionManager {
    sessions: Arc<Mutex<HashMap<SessionId, GameSession>>>,
    client_to_session: Arc<Mutex<HashMap<ClientId, SessionId>>>,
    broadcaster: Broadcaster,
    lobby_manager: LobbyManager,
}

impl std::fmt::Debug for GameSessionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameSessionManager").finish()
    }
}

impl GameSessionManager {
    pub fn new(broadcaster: Broadcaster, lobby_manager: LobbyManager) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            client_to_session: Arc::new(Mutex::new(HashMap::new())),
            broadcaster,
            lobby_manager,
        }
    }

    pub async fn remove_session(&self, session_id: &SessionId) {
        let mut sessions = self.sessions.lock().await;
        sessions.remove(session_id);
        drop(sessions);

        let mut mapping = self.client_to_session.lock().await;
        mapping.retain(|_, sid| sid != session_id);

        log!("Game session removed: {}", session_id);
    }

    pub async fn create_session(
        &self,
        session_id: SessionId,
        _lobby_details: common::LobbyDetails,
    ) {
        let lobby_id = LobbyId::new(session_id.clone());
        let lobby = match self.lobby_manager.get_lobby(&lobby_id).await {
            Some(l) => l,
            None => {
                log!("Cannot create session: lobby {} not found", session_id);
                return;
            }
        };

        let human_players: Vec<PlayerId> = lobby.players.keys().cloned().collect();
        let observers: HashSet<PlayerId> = lobby.observers.clone();
        let bots: HashMap<BotId, common::lobby::BotType> = lobby.bots.clone();

        let config = GameSessionConfig {
            session_id: session_id.clone(),
            human_players: human_players.clone(),
            observers,
            bots,
        };

        match &lobby.settings {
            LobbySettings::Snake(snake_settings) => {
                let settings = SnakeSessionSettings::from(snake_settings);
                let seed: u64 = rand::random();

                let players: Vec<common::PlayerIdentity> = config.human_players.iter()
                    .map(|p| common::PlayerIdentity { player_id: p.to_string(), is_bot: false })
                    .chain(config.bots.keys().map(|b| common::PlayerIdentity { player_id: b.to_player_id().to_string(), is_bot: true }))
                    .collect();

                let replay_recorder = Arc::new(Mutex::new(ReplayRecorder::new(
                    common::version::VERSION.to_string(),
                    ReplayGame::Snake,
                    seed,
                    Some(common::lobby_settings::Settings::Snake(snake_settings.clone())),
                    players,
                )));

                let session_state = create_snake_session(&config, &settings, Some(seed), Some(replay_recorder.clone()));

                self.register_snake_session(
                    session_id.clone(),
                    session_state.game_state.clone(),
                    session_state.tick.clone(),
                    replay_recorder,
                    &human_players,
                ).await;

                let manager = self.clone();
                let broadcaster = self.broadcaster.clone();
                let config_clone = config.clone();

                tokio::spawn(async move {
                    let notification = run_snake_game_loop(config_clone, session_state, broadcaster).await;
                    manager.handle_game_over(&config, notification).await;
                });
            }
            LobbySettings::TicTacToe(ttt_settings) => {
                let settings = TicTacToeSessionSettings::from(ttt_settings);
                let seed: u64 = rand::random();

                let players: Vec<common::PlayerIdentity> = config.human_players.iter()
                    .map(|p| common::PlayerIdentity { player_id: p.to_string(), is_bot: false })
                    .chain(config.bots.keys().map(|b| common::PlayerIdentity { player_id: b.to_player_id().to_string(), is_bot: true }))
                    .collect();

                let replay_recorder = Arc::new(Mutex::new(ReplayRecorder::new(
                    common::version::VERSION.to_string(),
                    ReplayGame::Tictactoe,
                    seed,
                    Some(common::lobby_settings::Settings::Tictactoe(ttt_settings.clone())),
                    players,
                )));

                match create_tictactoe_session(&config, &settings, Some(seed), Some(replay_recorder.clone())) {
                    Ok(session_state) => {
                        self.register_tictactoe_session(
                            session_id.clone(),
                            session_state.game_state.clone(),
                            session_state.turn_notify.clone(),
                            replay_recorder,
                            &human_players,
                        ).await;

                        let manager = self.clone();
                        let broadcaster = self.broadcaster.clone();
                        let config_clone = config.clone();

                        tokio::spawn(async move {
                            let notification = run_tictactoe_game_loop(config_clone, session_state, broadcaster).await;
                            manager.handle_game_over(&config, notification).await;
                        });
                    }
                    Err(e) => {
                        log!("Failed to create TicTacToe session: {}", e);
                    }
                }
            }
        }
    }

    async fn register_snake_session(
        &self,
        session_id: SessionId,
        game_state: Arc<Mutex<SnakeGameState>>,
        tick: Arc<Mutex<u64>>,
        replay_recorder: Arc<Mutex<ReplayRecorder>>,
        human_players: &[PlayerId],
    ) {
        let session = GameSession::Snake { game_state, tick, replay_recorder };

        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.clone(), session);
        drop(sessions);

        let mut mapping = self.client_to_session.lock().await;
        for player_id in human_players {
            mapping.insert(ClientId::new(player_id.to_string()), session_id.clone());
        }

        log!("Snake game session registered: {} with {} players", session_id, human_players.len());
    }

    async fn register_tictactoe_session(
        &self,
        session_id: SessionId,
        game_state: Arc<Mutex<TicTacToeGameState>>,
        turn_notify: Arc<Notify>,
        replay_recorder: Arc<Mutex<ReplayRecorder>>,
        human_players: &[PlayerId],
    ) {
        let session = GameSession::TicTacToe { game_state, turn_notify, replay_recorder };

        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.clone(), session);
        drop(sessions);

        let mut mapping = self.client_to_session.lock().await;
        for player_id in human_players {
            mapping.insert(ClientId::new(player_id.to_string()), session_id.clone());
        }

        log!("TicTacToe game session registered: {} with {} players", session_id, human_players.len());
    }

    async fn handle_game_over(&self, config: &GameSessionConfig, notification: common::GameOverNotification) {
        let client_ids: Vec<ClientId> = config.human_players.iter()
            .map(|p| ClientId::new(p.to_string()))
            .chain(config.observers.iter().map(|p| ClientId::new(p.to_string())))
            .collect();

        let game_over_msg = ServerMessage {
            message: Some(server_message::Message::GameOver(notification)),
        };

        self.broadcaster.broadcast_to_clients(&client_ids, game_over_msg).await;

        // Finalize and send replay
        if let Some(replay_notification) = self.finalize_replay(&config.session_id).await {
            let replay_msg = ServerMessage {
                message: Some(server_message::Message::ReplayFile(replay_notification)),
            };
            self.broadcaster.broadcast_to_clients(&client_ids, replay_msg).await;
        }

        let lobby_id = LobbyId::new(config.session_id.clone());
        match self.lobby_manager.end_game(&lobby_id).await {
            Ok(_player_ids) => {
                log!("Game ended for lobby {}, {} players in lobby", config.session_id, _player_ids.len());

                if let Some(_lobby_details) = self.lobby_manager.get_lobby_details(&lobby_id).await {
                    match self.lobby_manager.get_play_again_status(&lobby_id).await {
                        Ok(status) => {
                            let (ready_players, pending_players, available) = match status {
                                crate::lobby_manager::PlayAgainStatus::NotAvailable => {
                                    (vec![], vec![], false)
                                }
                                crate::lobby_manager::PlayAgainStatus::Available { ready_player_ids, pending_player_ids } => {
                                    let ready = ready_player_ids.iter().map(|id| common::PlayerIdentity {
                                        player_id: id.clone(),
                                        is_bot: false,
                                    }).collect();
                                    let pending = pending_player_ids.iter().map(|id| common::PlayerIdentity {
                                        player_id: id.clone(),
                                        is_bot: false,
                                    }).collect();
                                    (ready, pending, true)
                                }
                            };

                            let play_again_msg = ServerMessage {
                                message: Some(server_message::Message::PlayAgainStatus(
                                    common::PlayAgainStatusNotification {
                                        ready_players,
                                        pending_players,
                                        available,
                                    }
                                )),
                            };

                            self.broadcaster.broadcast_to_clients(&client_ids, play_again_msg).await;
                        }
                        Err(e) => {
                            log!("Failed to get play again status: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                log!("Failed to end game for lobby {}: {}", config.session_id, e);
            }
        }

        self.remove_session(&config.session_id).await;
    }

    async fn finalize_replay(&self, session_id: &SessionId) -> Option<ReplayFileReadyNotification> {
        let sessions = self.sessions.lock().await;
        let session = sessions.get(session_id)?;

        let (replay_recorder, game) = match session {
            GameSession::Snake { replay_recorder, .. } => (replay_recorder.clone(), ReplayGame::Snake),
            GameSession::TicTacToe { replay_recorder, .. } => (replay_recorder.clone(), ReplayGame::Tictactoe),
        };
        drop(sessions);

        let replay = {
            let mut recorder = replay_recorder.lock().await;
            recorder.finalize()
        };

        let suggested_file_name = generate_replay_filename(game, common::version::VERSION);
        let content = save_replay_to_bytes(&replay);

        log!("Replay finalized: {} ({} bytes, {} actions)", suggested_file_name, content.len(), replay.actions.len());

        Some(ReplayFileReadyNotification {
            version: REPLAY_VERSION as i32,
            suggested_file_name,
            content,
        })
    }

    pub async fn handle_snake_command(
        &self,
        client_id: &ClientId,
        command: InGameCommand,
    ) {
        let direction = match &command.command {
            Some(in_game_command::Command::Snake(snake_cmd)) => {
                match &snake_cmd.command {
                    Some(common::proto::snake::snake_in_game_command::Command::Turn(turn_cmd)) => {
                        match common::proto::snake::Direction::try_from(turn_cmd.direction) {
                            Ok(common::proto::snake::Direction::Up) => Direction::Up,
                            Ok(common::proto::snake::Direction::Down) => Direction::Down,
                            Ok(common::proto::snake::Direction::Left) => Direction::Left,
                            Ok(common::proto::snake::Direction::Right) => Direction::Right,
                            _ => return,
                        }
                    }
                    _ => return,
                }
            }
            _ => return,
        };

        let mapping = self.client_to_session.lock().await;
        let session_id = match mapping.get(client_id) {
            Some(id) => id.clone(),
            None => return,
        };
        drop(mapping);

        let sessions = self.sessions.lock().await;
        if let Some(GameSession::Snake { game_state, tick, replay_recorder }) = sessions.get(&session_id) {
            let game_state = game_state.clone();
            let tick = tick.clone();
            let replay_recorder = replay_recorder.clone();
            drop(sessions);

            let current_tick = *tick.lock().await;

            {
                let mut recorder = replay_recorder.lock().await;
                if let Some(player_index) = recorder.find_player_index(&client_id.to_string()) {
                    recorder.record_command(current_tick as i64, player_index, command);
                }
            }

            crate::games::snake::session::handle_direction(&game_state, client_id, direction).await;
        }
    }

    pub async fn kill_snake(
        &self,
        client_id: &ClientId,
        reason: DeathReason,
    ) {
        let mapping = self.client_to_session.lock().await;
        let session_id = match mapping.get(client_id) {
            Some(id) => id.clone(),
            None => return,
        };
        drop(mapping);

        let sessions = self.sessions.lock().await;
        if let Some(GameSession::Snake { game_state, tick, replay_recorder }) = sessions.get(&session_id) {
            let game_state = game_state.clone();
            let tick = tick.clone();
            let replay_recorder = replay_recorder.clone();
            drop(sessions);

            if reason == DeathReason::PlayerDisconnected {
                let current_tick = *tick.lock().await;
                let mut recorder = replay_recorder.lock().await;
                if let Some(player_index) = recorder.find_player_index(&client_id.to_string()) {
                    recorder.record_disconnect(current_tick as i64, player_index);
                }
            }

            crate::games::snake::session::handle_kill_snake(&game_state, client_id, reason).await;
        }
    }

    pub async fn handle_tictactoe_command(
        &self,
        client_id: &ClientId,
        command: InGameCommand,
    ) {
        let (x, y) = match &command.command {
            Some(in_game_command::Command::Tictactoe(ttt_cmd)) => {
                match &ttt_cmd.command {
                    Some(common::proto::tictactoe::tic_tac_toe_in_game_command::Command::Place(place_cmd)) => {
                        (place_cmd.x, place_cmd.y)
                    }
                    _ => return,
                }
            }
            _ => return,
        };

        let mapping = self.client_to_session.lock().await;
        let session_id = match mapping.get(client_id) {
            Some(id) => id.clone(),
            None => return,
        };
        drop(mapping);

        let sessions = self.sessions.lock().await;
        if let Some(GameSession::TicTacToe { game_state, turn_notify, replay_recorder }) = sessions.get(&session_id) {
            let game_state = game_state.clone();
            let turn_notify = turn_notify.clone();
            let replay_recorder = replay_recorder.clone();
            drop(sessions);

            {
                let mut recorder = replay_recorder.lock().await;
                let turn = recorder.actions_count() as i64;
                if let Some(player_index) = recorder.find_player_index(&client_id.to_string()) {
                    recorder.record_command(turn, player_index, command);
                }
            }

            if let Err(e) = crate::games::tictactoe::session::handle_place_mark(
                &game_state,
                &turn_notify,
                client_id,
                x,
                y,
            ).await {
                log!("Failed to place mark: {}", e);
            }
        }
    }
}

impl Clone for GameSessionManager {
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(),
            client_to_session: self.client_to_session.clone(),
            broadcaster: self.broadcaster.clone(),
            lobby_manager: self.lobby_manager.clone(),
        }
    }
}
