#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RowIndex(u32);

impl RowIndex {
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    pub(crate) fn from_usize(index: usize) -> Self {
        Self(u32::try_from(index).unwrap_or(u32::MAX))
    }
}
