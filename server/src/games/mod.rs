pub mod snake;
pub mod tictactoe;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use common::{PlayerId, BotId};
use crate::broadcaster::Broadcaster;
use crate::lobby_manager::BotType;

pub type SessionId = String;

#[derive(Debug)]
pub enum GameStateEnum {
    Snake(snake::GameState),
    TicTacToe(tictactoe::game_state::TicTacToeGameState),
}

pub struct GameSessionContext {
    pub session_id: SessionId,
    pub human_players: Vec<PlayerId>,
    pub observers: HashSet<PlayerId>,
    pub bots: HashMap<BotId, BotType>,
    pub broadcaster: Broadcaster,
}

pub type SharedContext = Arc<GameSessionContext>;

pub struct GameSessionResult {
    pub state: Arc<Mutex<GameStateEnum>>,
    pub tick: Arc<Mutex<u64>>,
    pub bots: Arc<Mutex<HashMap<BotId, BotType>>>,
    pub observers: Arc<Mutex<HashSet<PlayerId>>>,
}

pub struct GameOverResult {
    pub session_id: SessionId,
    pub scores: Vec<common::ScoreEntry>,
    pub winner: Option<common::PlayerIdentity>,
    pub game_info: common::game_over_notification::GameInfo,
    pub human_players: Vec<PlayerId>,
    pub observers: HashSet<PlayerId>,
}
