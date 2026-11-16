use crate::slot::Slot;

/// Whose input the game expects next; the authority uses it to schedule bots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Awaiting {
    Turn(Slot),
    /// Every seat still in play may act (real-time games).
    Simultaneous,
    /// No seat can act: the game is over, or it waits for the authority's world event (setup).
    Nobody,
}
