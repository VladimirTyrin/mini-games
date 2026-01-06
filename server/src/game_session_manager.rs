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
use crate::broadcaster::Broadcaster;
use crate::lobby_manager::{LobbyManager, LobbySettings};

pub type SessionId = String;

enum GameSession {
    Snake {
        game_state: Arc<Mutex<SnakeGameState>>,
    },
    TicTacToe {
        game_state: Arc<Mutex<TicTacToeGameState>>,
        turn_notify: Arc<Notify>,
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
                let session_state = create_snake_session(&config, &settings);

                self.register_snake_session(
                    session_id.clone(),
                    session_state.game_state.clone(),
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
                match create_tictactoe_session(&config, &settings) {
                    Ok(session_state) => {
                        self.register_tictactoe_session(
                            session_id.clone(),
                            session_state.game_state.clone(),
                            session_state.turn_notify.clone(),
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
        human_players: &[PlayerId],
    ) {
        let session = GameSession::Snake { game_state };

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
        human_players: &[PlayerId],
    ) {
        let session = GameSession::TicTacToe { game_state, turn_notify };

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

    pub async fn set_direction(
        &self,
        client_id: &ClientId,
        direction: Direction,
    ) {
        let mapping = self.client_to_session.lock().await;
        let session_id = match mapping.get(client_id) {
            Some(id) => id.clone(),
            None => return,
        };
        drop(mapping);

        let sessions = self.sessions.lock().await;
        if let Some(GameSession::Snake { game_state }) = sessions.get(&session_id) {
            crate::games::snake::session::handle_direction(game_state, client_id, direction).await;
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
        if let Some(GameSession::Snake { game_state }) = sessions.get(&session_id) {
            crate::games::snake::session::handle_kill_snake(game_state, client_id, reason).await;
        }
    }

    pub async fn place_mark(
        &self,
        client_id: &ClientId,
        x: u32,
        y: u32,
    ) {
        let mapping = self.client_to_session.lock().await;
        let session_id = match mapping.get(client_id) {
            Some(id) => id.clone(),
            None => return,
        };
        drop(mapping);

        let sessions = self.sessions.lock().await;
        if let Some(GameSession::TicTacToe { game_state, turn_notify }) = sessions.get(&session_id) {
            let game_state = game_state.clone();
            let turn_notify = turn_notify.clone();
            drop(sessions);

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
