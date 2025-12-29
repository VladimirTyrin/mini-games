use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;
use common::{ClientId, LobbyId, log, ServerMessage, server_message, GameStateUpdate, Position, ScoreEntry, GameOverNotification, GameEndReason};
use crate::game::{GameState, FieldSize, WallCollisionMode, Direction, Point, DeathReason};
use crate::broadcaster::Broadcaster;
use crate::lobby_manager::LobbyManager;

pub type SessionId = String;

#[derive(Debug)]
pub struct GameSessionManager {
    sessions: Arc<Mutex<HashMap<SessionId, GameSession>>>,
    client_to_session: Arc<Mutex<HashMap<ClientId, SessionId>>>,
    broadcaster: Broadcaster,
    lobby_manager: LobbyManager,
}

struct GameSession {
    state: Arc<Mutex<GameState>>,
    tick: Arc<Mutex<u64>>,
}

impl std::fmt::Debug for GameSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameSession")
            .field("state", &self.state)
            .field("tick", &self.tick)
            .finish()
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

    pub async fn get_session_for_client(&self, client_id: &ClientId) -> Option<SessionId> {
        let mapping = self.client_to_session.lock().await;
        mapping.get(client_id).cloned()
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
        lobby_details: common::LobbyDetails,
    ) {
        let player_ids: Vec<ClientId> = lobby_details.players.iter()
            .map(|p| ClientId::new(p.client_id.clone()))
            .collect();

        let settings = lobby_details.settings.unwrap_or_default();
        let field_width = settings.field_width as usize;
        let field_height = settings.field_height as usize;
        let wall_collision_mode = match common::WallCollisionMode::try_from(settings.wall_collision_mode) {
            Ok(common::WallCollisionMode::Death) |
            Ok(common::WallCollisionMode::Unspecified) => WallCollisionMode::Death,
            Ok(common::WallCollisionMode::WrapAround) => WallCollisionMode::WrapAround,
            _ => WallCollisionMode::Death,
        };
        let tick_interval = Duration::from_millis(settings.tick_interval_ms as u64);
        let mut sessions = self.sessions.lock().await;

        let field_size = FieldSize {
            width: field_width,
            height: field_height,
        };
        let mut game_state = GameState::new(field_size, wall_collision_mode);

        for (i, player_id) in player_ids.iter().enumerate() {
            let start_pos = Self::calculate_start_position(i, player_ids.len(), field_width, field_height);
            let direction = Self::calculate_start_direction(i, player_ids.len());
            game_state.add_snake(player_id.clone(), start_pos, direction);
        }

        let state = Arc::new(Mutex::new(game_state));
        let tick = Arc::new(Mutex::new(0u64));

        let state_clone = state.clone();
        let tick_clone = tick.clone();
        let session_id_clone = session_id.clone();
        let broadcaster_clone = self.broadcaster.clone();
        let lobby_manager_clone = self.lobby_manager.clone();
        let session_manager_clone = self.clone();
        let player_ids_clone = player_ids.clone();

        let _ = tokio::spawn(async move {
            let mut tick_interval_timer = interval(tick_interval);

            loop {
                tick_interval_timer.tick().await;

                let mut state = state_clone.lock().await;
                state.update();

                let mut tick_value = tick_clone.lock().await;
                *tick_value += 1;

                let mut snakes = vec![];
                for (id, snake) in &state.snakes {
                    let segments = snake.body.iter().map(|p| Position {
                        x: p.x as i32,
                        y: p.y as i32,
                    }).collect();

                    snakes.push(common::Snake {
                        client_id: id.to_string(),
                        segments,
                        alive: snake.is_alive(),
                        score: snake.score,
                    });
                }

                let food: Vec<Position> = state.food_set.iter().map(|p| Position {
                    x: p.x as i32,
                    y: p.y as i32,
                }).collect();

                let game_state_msg = ServerMessage {
                    message: Some(server_message::Message::State(
                        GameStateUpdate {
                            tick: *tick_value,
                            snakes,
                            food,
                            field_width: state.field_size.width as u32,
                            field_height: state.field_size.height as u32,
                        }
                    )),
                };

                broadcaster_clone.broadcast_to_clients(&player_ids_clone, game_state_msg).await;

                let alive_count = state.snakes.values().filter(|s| s.is_alive()).count();
                if alive_count <= 1 {
                    let scores: Vec<ScoreEntry> = state.snakes.iter().map(|(id, snake)| {
                        ScoreEntry {
                            client_id: id.to_string(),
                            score: snake.score,
                        }
                    }).collect();

                    let winner_id = state.snakes.iter()
                        .find(|(_, snake)| snake.is_alive())
                        .map(|(id, _)| id.to_string())
                        .unwrap_or_default();

                    let game_end_reason = state.game_end_reason
                        .map(|r| match r {
                            DeathReason::WallCollision => GameEndReason::WallCollision as i32,
                            DeathReason::SelfCollision => GameEndReason::SelfCollision as i32,
                            DeathReason::OtherSnakeCollision => GameEndReason::SnakeCollision as i32,
                            DeathReason::PlayerDisconnected => GameEndReason::PlayerDisconnected as i32,
                        })
                        .unwrap_or(GameEndReason::GameCompleted as i32);

                    let game_over_msg = ServerMessage {
                        message: Some(server_message::Message::GameOver(
                            GameOverNotification {
                                scores,
                                winner_id,
                                reason: game_end_reason,
                            }
                        )),
                    };

                    broadcaster_clone.broadcast_to_clients(&player_ids_clone, game_over_msg).await;

                    drop(state);
                    drop(tick_value);

                    let lobby_id = LobbyId::new(session_id_clone.clone());
                    match lobby_manager_clone.end_game(&lobby_id).await {
                        Ok(_player_ids) => {
                            log!("Game ended for lobby {}, {} players in lobby", lobby_id, _player_ids.len());

                            if let Some(_lobby_details) = lobby_manager_clone.get_lobby_details(&lobby_id).await {
                                match lobby_manager_clone.get_play_again_status(&lobby_id).await {
                                    Ok(status) => {
                                        let proto_status = match status {
                                            crate::lobby_manager::PlayAgainStatus::NotAvailable => {
                                                common::play_again_status_notification::Status::NotAvailable(
                                                    common::PlayAgainNotAvailable {}
                                                )
                                            }
                                            crate::lobby_manager::PlayAgainStatus::Available { ready_player_ids, pending_player_ids, .. } => {
                                                common::play_again_status_notification::Status::Available(
                                                    common::PlayAgainAvailable {
                                                        ready_player_ids,
                                                        pending_player_ids,
                                                    }
                                                )
                                            }
                                        };

                                        let play_again_msg = ServerMessage {
                                            message: Some(server_message::Message::PlayAgainStatus(
                                                common::PlayAgainStatusNotification {
                                                    status: Some(proto_status),
                                                }
                                            )),
                                        };

                                        broadcaster_clone.broadcast_to_clients(&player_ids_clone, play_again_msg).await;
                                    }
                                    Err(e) => {
                                        log!("Failed to get play again status: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log!("Failed to end game for lobby {}: {}", lobby_id, e);
                        }
                    }
                    break;
                }

                drop(state);
                drop(tick_value);
            }

            session_manager_clone.remove_session(&session_id_clone).await;
        });

        let session = GameSession {
            state,
            tick,
        };

        sessions.insert(session_id.clone(), session);
        drop(sessions);

        let mut mapping = self.client_to_session.lock().await;
        for player_id in &player_ids {
            mapping.insert(player_id.clone(), session_id.clone());
        }

        log!("Game session created: {} with {} players", session_id, player_ids.len());
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
                let mut state = session.state.lock().await;
                state.set_snake_direction(client_id, direction);
            }
        }
    }

    pub async fn kill_snake(
        &self,
        client_id: &ClientId,
        reason: crate::game::DeathReason,
    ) {
        let mapping = self.client_to_session.lock().await;
        if let Some(session_id) = mapping.get(client_id) {
            let sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get(session_id) {
                let mut state = session.state.lock().await;
                state.kill_snake(client_id, reason);
            }
        }
    }

    fn calculate_start_position(index: usize, total: usize, width: usize, height: usize) -> Point {
        let spacing = if total <= 2 {
            width / (total + 1)
        } else {
            width / total
        };

        let x = if total == 1 {
            width / 2
        } else {
            (index + 1) * spacing
        };

        let y = height / 2;

        Point::new(x.min(width - 1), y)
    }

    fn calculate_start_direction(_index: usize, _total: usize) -> Direction {
        Direction::Up
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
