use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use common::{ClientId, PlayerId};
use common::engine::tictactoe::TicTacToeGameState;

pub async fn handle_place_mark(
    game_state: &Arc<Mutex<TicTacToeGameState>>,
    turn_notify: &Arc<Notify>,
    client_id: &ClientId,
    x: u32,
    y: u32,
) -> Result<(), String> {
    let mut state_guard = game_state.lock().await;
    let player_id = PlayerId::new(client_id.to_string());
    state_guard.place_mark(&player_id, x as usize, y as usize)?;
    drop(state_guard);

    turn_notify.notify_one();
    Ok(())
}
