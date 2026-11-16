/// Team score: every seat shares it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Score(u32);

impl Score {
    pub const ZERO: Self = Self(0);

    pub const fn get(self) -> u32 {
        self.0
    }

    pub(crate) fn add(&mut self, points: u32) {
        self.0 = self.0.saturating_add(points);
    }
}
