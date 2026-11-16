use crate::model::TileValue;

/// Sum of the values of all tiles created by merges (legacy scoring).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Score(u64);

impl Score {
    pub const ZERO: Score = Score(0);

    pub fn get(self) -> u64 {
        self.0
    }

    pub(crate) fn plus_merge(self, merged: TileValue) -> Score {
        Score(self.0.saturating_add(u64::from(merged.get())))
    }

    pub(crate) fn plus(self, other: Score) -> Score {
        Score(self.0.saturating_add(other.0))
    }
}
