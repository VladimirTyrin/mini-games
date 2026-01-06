use std::sync::Arc;
use tokio::sync::Mutex;
use common::{ClientId, PlayerId};
use common::engine::snake::{GameState, Direction, DeathReason};

pub async fn handle_direction(
    state: &Arc<Mutex<GameState>>,
    client_id: &ClientId,
    direction: Direction,
) {
    let mut state_guard = state.lock().await;
    let player_id = PlayerId::new(client_id.to_string());
    state_guard.set_snake_direction(&player_id, direction);
}

pub async fn handle_kill_snake(
    state: &Arc<Mutex<GameState>>,
    client_id: &ClientId,
    reason: DeathReason,
) {
    let mut state_guard = state.lock().await;
    let player_id = PlayerId::new(client_id.to_string());
    state_guard.kill_snake(&player_id, reason);
}
