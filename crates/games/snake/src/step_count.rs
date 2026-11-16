/// Simultaneous moves played so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StepCount(u64);

impl StepCount {
    pub const ZERO: Self = Self(0);

    pub const fn new(steps: u64) -> Self {
        Self(steps)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub(crate) const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}
