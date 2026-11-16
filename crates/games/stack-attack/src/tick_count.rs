/// A number of game ticks; multiply by [`TICK_MS`](crate::rules::TICK_MS) for wall time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TickCount(u64);

impl TickCount {
    pub const ZERO: Self = Self(0);

    pub const fn new(ticks: u64) -> Self {
        Self(ticks)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub(crate) const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}
