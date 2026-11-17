use mg_core::Slot;

use crate::model::{Ship, ShotOutcome};

/// What the authority resolves: chance, and what only it knows (the fleets).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldEvent {
    /// The seat has placed its fleet; where stays with the authority.
    FleetPlaced(Slot),
    /// Both fleets are placed: who fires first.
    FirstShooterChosen(Slot),
    /// What the shot just fired found.
    ShotResolved(ShotOutcome),
    /// A seat's fleet, once the game is over.
    FleetRevealed { slot: Slot, ships: Vec<Ship> },
}
