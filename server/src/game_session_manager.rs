use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio::time::{interval, Duration};
use common::{ClientId, log};
use crate::game::{GameState, FieldSize, WallCollisionMode, Direction, Point};

pub type SessionId = String;

#[derive(Debug)]
pub struct GameSessionManager {
    sessions: Arc<Mutex<HashMap<SessionId, GameSession>>>,
    client_to_session: Arc<Mutex<HashMap<ClientId, SessionId>>>,
}

#[derive(Debug)]
struct GameSession {
    state: Arc<Mutex<GameState>>,
    tick: Arc<Mutex<u64>>
}

impl GameSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            client_to_session: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_session_for_client(&self, client_id: &ClientId) -> Option<SessionId> {
        let mapping = self.client_to_session.lock().await;
        mapping.get(client_id).cloned()
    }

    pub async fn create_session(
        &self,
        session_id: SessionId,
        player_ids: Vec<ClientId>,
        field_width: usize,
        field_height: usize,
        wall_collision_mode: WallCollisionMode,
    ) -> Result<(), String> {
        let mut sessions = self.sessions.lock().await;

        if sessions.contains_key(&session_id) {
            return Err("Session already exists".to_string());
        }

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
        let (_, mut state_broadcast_rx) = mpsc::channel::<()>(1);

        let state_clone = state.clone();
        let tick_clone = tick.clone();
        let session_id_clone = session_id.clone();

        let _ = tokio::spawn(async move {
            let mut tick_interval = interval(Duration::from_millis(200));

            loop {
                tokio::select! {
                    _ = tick_interval.tick() => {
                        let mut state = state_clone.lock().await;
                        state.update();

                        let mut tick_value = tick_clone.lock().await;
                        *tick_value += 1;

                        drop(state);
                        drop(tick_value);
                    }
                    _ = state_broadcast_rx.recv() => {
                        break;
                    }
                }
            }

            log!("Game loop ended for session: {}", session_id_clone);
        });

        let session = GameSession {
            state,
            tick
        };

        sessions.insert(session_id.clone(), session);
        drop(sessions);

        let mut mapping = self.client_to_session.lock().await;
        for player_id in &player_ids {
            mapping.insert(player_id.clone(), session_id.clone());
        }

        log!("Game session created: {} with {} players", session_id, player_ids.len());

        Ok(())
    }

    pub async fn set_direction(
        &self,
        session_id: &SessionId,
        client_id: &ClientId,
        direction: Direction,
    ) -> Result<(), String> {
        let sessions = self.sessions.lock().await;

        let session = sessions.get(session_id)
            .ok_or_else(|| "Session not found".to_string())?;

        let mut state = session.state.lock().await;
        state.set_snake_direction(client_id, direction);

        Ok(())
    }

    pub async fn get_state(&self, session_id: &SessionId) -> Option<(GameState, u64)> {
        let sessions = self.sessions.lock().await;

        if let Some(session) = sessions.get(session_id) {
            let state = session.state.lock().await;
            let tick = session.tick.lock().await;
            Some((state.clone(), *tick))
        } else {
            None
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

    pub async fn is_game_over(&self, session_id: &SessionId) -> bool {
        let sessions = self.sessions.lock().await;

        if let Some(session) = sessions.get(session_id) {
            let state = session.state.lock().await;
            let alive_count = state.snakes.values().filter(|s| s.alive).count();
            alive_count <= 1
        } else {
            true
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
        }
    }
}
