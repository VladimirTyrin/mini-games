/// Why a set of ships is not a fleet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FleetError {
    /// Not one ship of four, two of three, three of two and four of one.
    WrongShips,
    /// Two ships share a cell, a side or a corner.
    ShipsTouch,
}

impl FleetError {
    pub fn reason(self) -> &'static str {
        match self {
            FleetError::WrongShips => {
                "a fleet is one ship of 4, two of 3, three of 2 and four of 1"
            }
            FleetError::ShipsTouch => "ships may not touch, not even at a corner",
        }
    }
}
