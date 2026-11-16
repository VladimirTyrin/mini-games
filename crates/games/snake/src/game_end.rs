use mg_core::Slot;

use crate::death_reason::DeathReason;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GameEnd {
    /// Reason of the death that ended the match; simultaneous deaths report the lowest seat's.
    pub reason: DeathReason,
    /// The sole survivor of a multi-seat match; `None` for a draw and for solo matches.
    pub winner: Option<Slot>,
}
