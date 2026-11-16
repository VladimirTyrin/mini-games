/// Food eaten by one snake.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Score(u32);

impl Score {
    pub const ZERO: Score = Score(0);

    pub const fn new(food_eaten: u32) -> Self {
        Self(food_eaten)
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    pub(crate) fn increment(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
}
