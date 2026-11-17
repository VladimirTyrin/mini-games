use super::Bound;

#[derive(Clone, Copy, Debug)]
pub(crate) struct TableEntry {
    pub(crate) key: u64,
    pub(crate) score: i32,
    pub(crate) bound: Bound,
    /// Plies and capture extension left when the score was found; a shallower search may use it.
    pub(crate) depth: u8,
    pub(crate) extension: u8,
    /// Index into the position's legal moves (generation order) of the best move found, tried
    /// first next time.
    pub(crate) best_move: u8,
}
