/// Assigned in spawn order starting at 1, never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CraneId(u32);

impl CraneId {
    pub(crate) const FIRST: Self = Self(1);

    pub const fn get(self) -> u32 {
        self.0
    }

    pub(crate) const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}
