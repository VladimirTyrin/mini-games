use std::fmt;

use mg_core::EnglishText;

/// Why a shot or a fleet is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    BattleNotStarted,
    LastShotNotResolved,
    CellAlreadyKnown,
    FleetAlreadyPlaced,
    WrongShips,
    ShipsTouch,
    // Input no honest client or authority sends: a world event that does not fit the state, a
    // seat or cell that does not exist. Clients show it as a generic "not allowed".
    FleetCannotBePlacedNow,
    NoShotPending,
    NoSuchSea,
    SunkShipDoesNotFit,
    FirstShooterAlreadyChosen,
    FleetRevealedEarly,
    RevealedFleetInvalid,
}

impl EnglishText for Refusal {
    fn english(self) -> &'static str {
        match self {
            Self::BattleNotStarted => "the battle has not started yet",
            Self::LastShotNotResolved => "the last shot is not resolved yet",
            Self::CellAlreadyKnown => "that cell is known already",
            Self::FleetAlreadyPlaced => "the fleet is placed already",
            Self::WrongShips => "a fleet is one ship of 4, two of 3, three of 2 and four of 1",
            Self::ShipsTouch => "ships may not touch, not even at a corner",
            Self::FleetCannotBePlacedNow => "this fleet cannot be placed now",
            Self::NoShotPending => "no shot is being fired",
            Self::NoSuchSea => "no such sea",
            Self::SunkShipDoesNotFit => "that ship was not hit there",
            Self::FirstShooterAlreadyChosen => "the first shooter is already chosen",
            Self::FleetRevealedEarly => "fleets are revealed once, at the end",
            Self::RevealedFleetInvalid => "a revealed fleet must be a fleet",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.english())
    }
}
