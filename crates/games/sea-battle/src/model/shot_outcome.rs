use crate::model::Ship;

/// What a shot found; only the authority, which knows the fleets, decides it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShotOutcome {
    Miss,
    Hit,
    /// The shot hit the last afloat cell of this ship.
    Sunk(Ship),
}
