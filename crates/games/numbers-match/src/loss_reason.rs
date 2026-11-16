#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LossReason {
    /// No removable pair and no refills left.
    NoMoves,
    Dropped,
}
