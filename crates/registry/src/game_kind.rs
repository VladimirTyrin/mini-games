use mg_core::{Game, GameInfo};
use mg_numbers_match::NumbersMatch;
use mg_protocol::proto::game_settings;
use mg_protocol::{WireError, proto};
use mg_puzzle2048::Puzzle2048;
use mg_snake::Snake;
use mg_stack_attack::StackAttack;
use mg_tictactoe::TicTacToe;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameKind {
    Snake,
    TicTacToe,
    StackAttack,
    Puzzle2048,
    NumbersMatch,
}

impl GameKind {
    pub const ALL: [GameKind; 5] = [
        GameKind::Snake,
        GameKind::TicTacToe,
        GameKind::StackAttack,
        GameKind::Puzzle2048,
        GameKind::NumbersMatch,
    ];

    pub fn of_settings(settings: &proto::GameSettings) -> Result<Self, WireError> {
        match settings
            .game
            .as_ref()
            .ok_or(WireError::Missing("settings.game"))?
        {
            game_settings::Game::Snake(_) => Ok(Self::Snake),
            game_settings::Game::Tictactoe(_) => Ok(Self::TicTacToe),
            game_settings::Game::StackAttack(_) => Ok(Self::StackAttack),
            game_settings::Game::Puzzle2048(_) => Ok(Self::Puzzle2048),
            game_settings::Game::NumbersMatch(_) => Ok(Self::NumbersMatch),
        }
    }

    pub fn info(self) -> GameInfo {
        match self {
            Self::Snake => Snake::INFO,
            Self::TicTacToe => TicTacToe::INFO,
            Self::StackAttack => StackAttack::INFO,
            Self::Puzzle2048 => Puzzle2048::INFO,
            Self::NumbersMatch => NumbersMatch::INFO,
        }
    }
}
