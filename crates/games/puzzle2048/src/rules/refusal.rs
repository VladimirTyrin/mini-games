use std::fmt;

use mg_core::EnglishText;

/// Why a move is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    MoveChangesNothing,
    // Input no honest client or authority sends: a world event that does not fit the state, a
    // seat or cell that does not exist. Clients show it as a generic "not allowed".
    NoSpawnDue,
    SpawnCellOutside,
    SpawnCellOccupied,
}

impl EnglishText for Refusal {
    fn english(self) -> &'static str {
        match self {
            Self::MoveChangesNothing => "the move does not change the board",
            Self::NoSpawnDue => "no tile spawn is due",
            Self::SpawnCellOutside => "spawn cell is outside the board",
            Self::SpawnCellOccupied => "spawn cell is occupied",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.english())
    }
}
