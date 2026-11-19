use std::fmt;

use mg_core::EnglishText;

/// Why a removal, refill or hint is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    NotDealtYet,
    SameCell,
    CellCrossedOut,
    NotAPair,
    TooFarApart,
    NoRefillsLeft,
    HintsDisabled,
    NoHintsLeft,
    HintAlreadyShown,
    // Input no honest client or authority sends: a world event that does not fit the state, a
    // seat or cell that does not exist. Clients show it as a generic "not allowed".
    DealWrongSize,
    DealHasAdjacentTwins,
    AlreadyDealt,
}

impl EnglishText for Refusal {
    fn english(self) -> &'static str {
        match self {
            Self::NotDealtYet => "the board is not dealt yet",
            Self::SameCell => "pick two different cells",
            Self::CellCrossedOut => "that cell is already crossed out",
            Self::NotAPair => "not a pair: equal numbers or a sum of 10",
            Self::TooFarApart => {
                "too far apart: pairs must touch, or have only crossed-out cells between them"
            }
            Self::NoRefillsLeft => "no refills left",
            Self::HintsDisabled => "hints are disabled",
            Self::NoHintsLeft => "no hints left",
            Self::HintAlreadyShown => "a hint is already shown",
            Self::DealWrongSize => "the deal must have 42 values",
            Self::DealHasAdjacentTwins => "dealt row has equal adjacent values",
            Self::AlreadyDealt => "the board is already dealt",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.english())
    }
}
