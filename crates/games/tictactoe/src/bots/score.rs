use super::side::Side;

/// Bot-perspective minimax value. Terminal values (`±(WIN - ply)`) always dominate heuristic
/// values, which are clamped to `±HEURISTIC_LIMIT`, so the search never trades a forced loss for
/// a nicer-looking position (legacy T4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Score(i64);

impl Score {
    const WIN: i64 = i64::MAX / 4;
    pub(crate) const HEURISTIC_LIMIT: i64 = Self::WIN / 4;
    /// Deeper than any search can reach (at most 400 plies), so `WIN - ply` stays decisive.
    const DECISIVE: i64 = Self::WIN - 1_000;

    pub(crate) const MIN: Score = Score(i64::MIN);
    pub(crate) const MAX: Score = Score(i64::MAX);
    pub(crate) const DRAW: Score = Score(0);

    pub(crate) fn heuristic(evaluation: i64) -> Score {
        Score(evaluation.clamp(-Self::HEURISTIC_LIMIT, Self::HEURISTIC_LIMIT))
    }

    /// `side` completes a line with the move at `ply` (the root move is ply 1). Faster wins and
    /// slower losses score better.
    pub(crate) fn win_for(side: Side, ply: u32) -> Score {
        let value = Self::WIN - i64::from(ply);
        match side {
            Side::Bot => Score(value),
            Side::Opponent => Score(-value),
        }
    }

    pub(crate) fn is_decisive(self) -> bool {
        self.0.unsigned_abs() >= Self::DECISIVE.unsigned_abs()
    }

    #[cfg(test)]
    pub(crate) fn value(self) -> i64 {
        self.0
    }
}
