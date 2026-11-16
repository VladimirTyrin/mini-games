use crate::slot::Slot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Viewer {
    Player(Slot),
    /// Observer or replay viewer.
    Spectator,
}
