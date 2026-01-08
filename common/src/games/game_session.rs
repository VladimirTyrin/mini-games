use std::sync::Arc;
use tokio::sync::Mutex;

use crate::ReplayGame;
use crate::games::snake::SnakeSessionState;
use crate::games::tictactoe::TicTacToeSessionState;
use crate::replay::ReplayRecorder;

#[derive(Clone)]
pub enum GameSession {
    Snake(SnakeSessionState),
    TicTacToe(TicTacToeSessionState),
}

impl GameSession {
    pub fn game_type(&self) -> ReplayGame {
        match self {
            GameSession::Snake(_) => ReplayGame::Snake,
            GameSession::TicTacToe(_) => ReplayGame::Tictactoe,
        }
    }

    pub fn replay_recorder(&self) -> Option<Arc<Mutex<ReplayRecorder>>> {
        match self {
            GameSession::Snake(state) => state.replay_recorder.clone(),
            GameSession::TicTacToe(state) => state.replay_recorder.clone(),
        }
    }

    pub fn snake_state(&self) -> Option<&SnakeSessionState> {
        match self {
            GameSession::Snake(state) => Some(state),
            _ => None,
        }
    }

    pub fn tictactoe_state(&self) -> Option<&TicTacToeSessionState> {
        match self {
            GameSession::TicTacToe(state) => Some(state),
            _ => None,
        }
    }
}
