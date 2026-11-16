#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutcomeKind {
    /// Competitive game; see [`crate::Outcome::winners`].
    Winner,
    Draw,
    /// Solo or cooperative game completed.
    Victory,
    /// Solo or cooperative game lost (score games always end this way).
    Defeat,
}
