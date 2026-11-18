use mg_core::Slot;

use crate::view::CrashPoint;

/// Where snakes crashed, for showing it: the rules only record that they died and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Crash {
    pub at: CrashPoint,
    /// In seat order.
    pub slots: Vec<Slot>,
    /// Two or more snakes met there: heads into one cell, or into each other.
    pub head_on: bool,
}
