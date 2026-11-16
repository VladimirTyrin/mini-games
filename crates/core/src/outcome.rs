use crate::slot::Slot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutcomeKind {
    /// Competitive game; see [`Outcome::winners`].
    Winner,
    Draw,
    /// Solo or cooperative game completed.
    Victory,
    /// Solo or cooperative game lost (score games always end this way).
    Defeat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub kind: OutcomeKind,
    /// Non-empty only for [`OutcomeKind::Winner`].
    pub winners: Vec<Slot>,
    /// Indexed by [`Slot::index`].
    pub scores: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    InProgress,
    Finished(Outcome),
}

impl Status {
    pub fn is_finished(&self) -> bool {
        matches!(self, Status::Finished(_))
    }
}
