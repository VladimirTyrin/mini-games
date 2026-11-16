/// Assigned in drop order starting at 1, never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoxId(u32);

impl BoxId {
    pub(crate) const FIRST: Self = Self(1);

    pub const fn get(self) -> u32 {
        self.0
    }

    pub(crate) const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}
