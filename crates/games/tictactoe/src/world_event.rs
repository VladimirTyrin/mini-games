use mg_core::Slot;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorldEvent {
    /// [`crate::FirstPlayerMode::Random`] only: this seat plays X and moves first.
    FirstPlayerChosen { x_player: Slot },
}
