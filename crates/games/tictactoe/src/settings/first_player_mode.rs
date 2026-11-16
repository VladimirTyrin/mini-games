#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FirstPlayerMode {
    /// The authority picks a seat and announces it as [`crate::WorldEvent::FirstPlayerChosen`]
    /// in the very first frame; until then nobody may move.
    Random,
    /// The lobby host (slot 0) plays X and moves first.
    Host,
    /// The other seat (slot 1) plays X and moves first.
    Guest,
}
