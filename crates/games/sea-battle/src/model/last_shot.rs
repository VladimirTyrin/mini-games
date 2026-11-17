use mg_core::Slot;

use crate::model::{Cell, ShotOutcome};

/// The latest shot, for the board to show it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LastShot {
    pub by: Slot,
    pub cell: Cell,
    pub outcome: ShotOutcome,
}
