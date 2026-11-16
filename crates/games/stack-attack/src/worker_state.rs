#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkerState {
    Grounded,
    /// Just jumped: hangs one tick before falling.
    Jumping,
    Falling,
}
