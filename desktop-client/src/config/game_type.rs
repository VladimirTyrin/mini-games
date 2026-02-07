use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
pub enum GameType {
    Snake,
    TicTacToe,
    NumbersMatch,
    Puzzle2048,
}
