use crate::model::CellIndex;

/// Ordered lexicographically (`first`, then `second`). Pairs produced by the game (hints,
/// [`crate::NumbersMatch::find_pair`]) always have `first < second`; commands may use any order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellPair {
    pub first: CellIndex,
    pub second: CellIndex,
}

impl CellPair {
    pub const fn new(first: CellIndex, second: CellIndex) -> Self {
        Self { first, second }
    }
}
