/// Cell position in reading order, `row * COLUMNS + column`, in the board as it is at that moment
/// (indices shift when rows are deleted).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellIndex(u32);

impl CellIndex {
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    pub(crate) fn from_usize(index: usize) -> Self {
        Self(u32::try_from(index).unwrap_or(u32::MAX))
    }

    pub(crate) const fn as_usize(self) -> usize {
        self.0 as usize
    }
}
