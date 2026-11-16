#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkerStatus {
    Alive,
    Crushed,
    /// The player left; the others play on (A11).
    Left,
}
