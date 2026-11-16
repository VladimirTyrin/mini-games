use mg_core::Slot;

use crate::winning_line::WinningLine;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameEnd {
    Win {
        winner: Slot,
        line: WinningLine,
    },
    /// The board filled up without a line.
    Draw,
    /// The other seat left play (disconnect budget exhausted, left, kicked).
    OpponentLeft {
        winner: Slot,
    },
}

impl GameEnd {
    pub fn winner(self) -> Option<Slot> {
        match self {
            GameEnd::Win { winner, .. } | GameEnd::OpponentLeft { winner } => Some(winner),
            GameEnd::Draw => None,
        }
    }
}
