use mg_core::Slot;

/// Randomness is resolved by the authority and sent to everyone as a world event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldEvent {
    /// Who plays white (and so moves first), drawn by lot.
    WhiteChosen(Slot),
}
