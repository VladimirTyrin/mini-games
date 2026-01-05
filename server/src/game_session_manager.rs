use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Notify};
use common::{ClientId, LobbyId, PlayerId, BotId, log, ServerMessage, server_message};
use crate::games::snake::{Direction, DeathReason};
use crate::games::{GameStateEnum, GameSessionContext, GameSessionResult, GameOverResult, SessionId};
use crate::broadcaster::Broadcaster;
use crate::lobby_manager::{LobbyManager, BotType, LobbySettings};

#[derive(Debug)]
pub struct GameSessionManager {
    sessions: Arc<Mutex<HashMap<SessionId, GameSession>>>,
    client_to_session: Arc<Mutex<HashMap<ClientId, SessionId>>>,
    tictactoe_notifies: Arc<Mutex<HashMap<SessionId, Arc<Notify>>>>,
    broadcaster: Broadcaster,
    lobby_manager: LobbyManager,
}

struct GameSession {
    state: Arc<Mutex<GameStateEnum>>,
    tick: Arc<Mutex<u64>>,
    bots: Arc<Mutex<HashMap<BotId, BotType>>>,
    observers: Arc<Mutex<HashSet<PlayerId>>>,
}

impl std::fmt::Debug for GameSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameSession")
            .field("state", &self.state)
            .field("tick", &self.tick)
            .field("bots", &self.bots)
            .field("observers", &self.observers)
            .finish()
    }
}

impl GameSessionManager {
    pub fn new(broadcaster: Broadcaster, lobby_manager: LobbyManager) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            client_to_session: Arc::new(Mutex::new(HashMap::new())),
            tictactoe_notifies: Arc::new(Mutex::new(HashMap::new())),
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
        drop(mapping);

        let mut notifies = self.tictactoe_notifies.lock().await;
        notifies.remove(session_id);

        log!("Game session removed: {}", session_id);
    }

    async fn register_tictactoe_notify(&self, session_id: SessionId, notify: Arc<Notify>) {
        let mut notifies = self.tictactoe_notifies.lock().await;
        notifies.insert(session_id, notify);
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
        let bots: HashMap<BotId, BotType> = lobby.bots.clone();

        let ctx = Arc::new(GameSessionContext {
            session_id: session_id.clone(),
            human_players: human_players.clone(),
            observers,
            bots,
            broadcaster: self.broadcaster.clone(),
        });

        match &lobby.settings {
            LobbySettings::Snake(snake_settings) => {
                let result = crate::games::snake::session::create_session(&ctx, snake_settings);
                let tick_interval = Duration::from_millis(snake_settings.tick_interval_ms as u64);

                self.register_session(session_id, &result, &human_players).await;

                let manager = self.clone();
                let state = result.state;
                let tick = result.tick;
                let bots = result.bots;
                let observers = result.observers;

                tokio::spawn(async move {
                    let game_over = crate::games::snake::session::run_game_loop(
                        ctx,
                        state,
                        tick,
                        bots,
                        observers,
                        tick_interval,
                    ).await;
                    manager.handle_game_over(game_over).await;
                });
            }
            LobbySettings::TicTacToe(ttt_settings) => {
                match crate::games::tictactoe::session::create_session(&ctx, ttt_settings) {
                    Ok(handle) => {
                        self.register_session(session_id.clone(), &handle.result, &human_players).await;
                        self.register_tictactoe_notify(session_id.clone(), handle.turn_notify.clone()).await;

                        let manager = self.clone();
                        let state = handle.result.state;
                        let bots = handle.result.bots;
                        let observers = handle.result.observers;
                        let turn_notify = handle.turn_notify;

                        tokio::spawn(async move {
                            let game_over = crate::games::tictactoe::session::run_game_loop(
                                ctx,
                                state,
                                bots,
                                observers,
                                turn_notify,
                            ).await;
                            manager.handle_game_over(game_over).await;
                        });
                    }
                    Err(e) => {
                        log!("Failed to create TicTacToe session: {}", e);
                    }
                }
            }
        }
    }

    async fn register_session(
        &self,
        session_id: SessionId,
        result: &GameSessionResult,
        human_players: &[PlayerId],
    ) {
        let session = GameSession {
            state: result.state.clone(),
            tick: result.tick.clone(),
            bots: result.bots.clone(),
            observers: result.observers.clone(),
        };

        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.clone(), session);
        drop(sessions);

        let mut mapping = self.client_to_session.lock().await;
        for player_id in human_players {
            mapping.insert(ClientId::new(player_id.to_string()), session_id.clone());
        }

        log!("Game session registered: {} with {} players", session_id, human_players.len());
    }

    async fn handle_game_over(&self, result: GameOverResult) {
        let client_ids: Vec<ClientId> = result.human_players.iter()
            .map(|p| ClientId::new(p.to_string()))
            .chain(result.observers.iter().map(|p| ClientId::new(p.to_string())))
            .collect();

        let game_over_msg = ServerMessage {
            message: Some(server_message::Message::GameOver(
                common::GameOverNotification {
                    scores: result.scores,
                    winner: result.winner,
                    game_info: Some(result.game_info),
                }
            )),
        };

        self.broadcaster.broadcast_to_clients(&client_ids, game_over_msg).await;

        let lobby_id = LobbyId::new(result.session_id.clone());
        match self.lobby_manager.end_game(&lobby_id).await {
            Ok(_player_ids) => {
                log!("Game ended for lobby {}, {} players in lobby", result.session_id, _player_ids.len());

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
                log!("Failed to end game for lobby {}: {}", result.session_id, e);
            }
        }

        self.remove_session(&result.session_id).await;
    }

    pub async fn set_direction(
        &self,
        client_id: &ClientId,
        direction: Direction,
    ) {
        let mapping = self.client_to_session.lock().await;
        if let Some(session_id) = mapping.get(client_id) {
            let sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get(session_id) {
                crate::games::snake::session::handle_direction(&session.state, client_id, direction).await;
            }
        }
    }

    pub async fn kill_snake(
        &self,
        client_id: &ClientId,
        reason: DeathReason,
    ) {
        let mapping = self.client_to_session.lock().await;
        if let Some(session_id) = mapping.get(client_id) {
            let sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get(session_id) {
                crate::games::snake::session::handle_kill_snake(&session.state, client_id, reason).await;
            }
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
        let (state_arc, bots_arc, observers_arc) = match sessions.get(&session_id) {
            Some(session) => (session.state.clone(), session.bots.clone(), session.observers.clone()),
            None => return,
        };
        drop(sessions);

        let notifies = self.tictactoe_notifies.lock().await;
        let turn_notify = match notifies.get(&session_id) {
            Some(n) => n.clone(),
            None => return,
        };
        drop(notifies);

        let human_players: Vec<PlayerId> = match self.lobby_manager.get_lobby(&LobbyId::new(session_id)).await {
            Some(lobby) => lobby.players.keys().cloned().collect(),
            None => vec![],
        };

        if let Err(e) = crate::games::tictactoe::session::handle_place_mark(
            &state_arc,
            &bots_arc,
            &observers_arc,
            &human_players,
            &self.broadcaster,
            &turn_notify,
            client_id,
            x,
            y,
        ).await {
            log!("Failed to place mark: {}", e);
        }
    }
}

impl Clone for GameSessionManager {
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(),
            client_to_session: self.client_to_session.clone(),
            tictactoe_notifies: self.tictactoe_notifies.clone(),
            broadcaster: self.broadcaster.clone(),
            lobby_manager: self.lobby_manager.clone(),
        }
    }
}
