use std::fmt;

use mg_core::EnglishText;

/// Why a turn is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The snake already heads (or will, after its queued turns) that way.
    AlreadyHeadingThatWay,
    /// A snake cannot turn back onto itself.
    CannotReverse,
    // Input no honest client or authority sends: a world event that does not fit the state, a
    // seat or cell that does not exist. Clients show it as a generic "not allowed".
    FoodCapReached,
    FoodCellNotEmpty,
}

impl EnglishText for Refusal {
    fn english(self) -> &'static str {
        match self {
            Self::AlreadyHeadingThatWay => "already heading that way",
            Self::CannotReverse => "cannot reverse",
            Self::FoodCapReached => "food cap reached",
            Self::FoodCellNotEmpty => "food needs an empty cell",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.english())
    }
}
