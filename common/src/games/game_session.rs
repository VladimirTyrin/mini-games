use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{ClientId, ReplayGame};
use crate::games::numbers_match::NumbersMatchSessionState;
use crate::games::puzzle2048::Puzzle2048SessionState;
use crate::games::snake::SnakeSessionState;
use crate::games::stack_attack::StackAttackSessionState;
use crate::games::tictactoe::TicTacToeSessionState;
use crate::replay::ReplayRecorder;

pub trait SessionState {
    fn tick(&self) -> &Arc<Mutex<u64>>;
    fn replay_recorder(&self) -> Option<&Arc<Mutex<ReplayRecorder>>>;
}

macro_rules! impl_session_state {
    ($($type:ty),+) => {
        $(
            impl SessionState for $type {
                fn tick(&self) -> &Arc<Mutex<u64>> {
                    &self.tick
                }
                fn replay_recorder(&self) -> Option<&Arc<Mutex<ReplayRecorder>>> {
                    self.replay_recorder.as_ref()
                }
            }
        )+
    };
}

impl_session_state!(
    SnakeSessionState,
    TicTacToeSessionState,
    NumbersMatchSessionState,
    StackAttackSessionState,
    Puzzle2048SessionState
);

#[derive(Clone)]
pub enum GameSession {
    Snake(SnakeSessionState),
    TicTacToe(TicTacToeSessionState),
    NumbersMatch(NumbersMatchSessionState),
    StackAttack(StackAttackSessionState),
    Puzzle2048(Puzzle2048SessionState),
}

impl GameSession {
    fn state(&self) -> &dyn SessionState {
        match self {
            GameSession::Snake(s) => s,
            GameSession::TicTacToe(s) => s,
            GameSession::NumbersMatch(s) => s,
            GameSession::StackAttack(s) => s,
            GameSession::Puzzle2048(s) => s,
        }
    }

    pub fn game_type(&self) -> ReplayGame {
        match self {
            GameSession::Snake(_) => ReplayGame::Snake,
            GameSession::TicTacToe(_) => ReplayGame::Tictactoe,
            GameSession::NumbersMatch(_) => ReplayGame::NumbersMatch,
            GameSession::StackAttack(_) => ReplayGame::StackAttack,
            GameSession::Puzzle2048(_) => ReplayGame::Puzzle2048,
        }
    }

    pub fn replay_recorder(&self) -> Option<Arc<Mutex<ReplayRecorder>>> {
        self.state().replay_recorder().cloned()
    }

    pub async fn current_tick(&self) -> u64 {
        *self.state().tick().lock().await
    }

    pub async fn record_disconnect(&self, client_id: &ClientId) {
        let Some(recorder) = self.state().replay_recorder() else {
            return;
        };

        let tick = *self.state().tick().lock().await;

        let mut recorder = recorder.lock().await;
        if let Some(player_index) = recorder.find_player_index(client_id.as_str()) {
            recorder.record_disconnect(tick as i64, player_index);
        }
    }
}
