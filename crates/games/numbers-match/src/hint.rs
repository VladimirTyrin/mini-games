use crate::cell_pair::CellPair;

/// Shown until the next pair removal or refill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hint {
    /// The first removable pair in reading order, so hints are deterministic.
    Pair(CellPair),
    /// No pair exists, but a refill is left.
    SuggestRefill,
}
