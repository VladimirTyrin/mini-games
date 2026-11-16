use mg_core::Slot;

use crate::rules::DeathReason;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GameEnd {
    /// Reason of the death that ended the match; simultaneous deaths report the lowest seat's.
    pub reason: DeathReason,
    /// Seats alive when a multi-seat match ended: the sole survivor, or every bot that outlived
    /// the last human. Empty for a draw and for solo matches.
    pub winners: Vec<Slot>,
}
