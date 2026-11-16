#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EndReason {
    /// No worker is alive and the last one was crushed.
    WorkersCrushed,
    /// A resting box is in the top row after line clears (A6).
    CeilingReached,
    /// No worker is alive and the last one left.
    AllWorkersLeft,
}
