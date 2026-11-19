use crate::rules::Refusal;

/// Why a set of ships is not a fleet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FleetError {
    /// Not one ship of four, two of three, three of two and four of one.
    WrongShips,
    /// Two ships share a cell, a side or a corner.
    ShipsTouch,
}

impl From<FleetError> for Refusal {
    fn from(error: FleetError) -> Self {
        match error {
            FleetError::WrongShips => Refusal::WrongShips,
            FleetError::ShipsTouch => Refusal::ShipsTouch,
        }
    }
}
