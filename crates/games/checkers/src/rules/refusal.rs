use std::fmt;

use mg_core::EnglishText;

/// Why a move is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A capture is available, and capturing is compulsory.
    MustCapture,
    NotALegalMove,
    // Input no honest client or authority sends: a world event that does not fit the state, a
    // seat or cell that does not exist. Clients show it as a generic "not allowed".
    WhiteAlreadyKnown,
    NoSuchSeat,
}

impl EnglishText for Refusal {
    fn english(self) -> &'static str {
        match self {
            Self::MustCapture => "you must capture",
            Self::NotALegalMove => "not a legal move",
            Self::WhiteAlreadyKnown => "who plays white is already known",
            Self::NoSuchSeat => "no such seat",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.english())
    }
}
