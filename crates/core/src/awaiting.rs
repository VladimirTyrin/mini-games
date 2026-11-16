use crate::slot::Slot;

/// Whose input the game expects next; the authority uses it to schedule bots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Awaiting {
    Turn(Slot),
    /// Every seat still in play may act (real-time games).
    Simultaneous,
    /// The game is over.
    Nobody,
}
