use std::fmt;

use mg_core::EnglishText;

/// Why a move is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    CellTaken,
    // Input no honest client or authority sends: a world event that does not fit the state, a
    // seat or cell that does not exist. Clients show it as a generic "not allowed".
    FirstPlayerAlreadyKnown,
    NoSuchSeat,
    CellOutsideBoard,
}

impl EnglishText for Refusal {
    fn english(self) -> &'static str {
        match self {
            Self::CellTaken => "cell is already marked",
            Self::FirstPlayerAlreadyKnown => "the first player is already known",
            Self::NoSuchSeat => "no such seat",
            Self::CellOutsideBoard => "cell is outside the board",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.english())
    }
}
