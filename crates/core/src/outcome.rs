use crate::outcome_kind::OutcomeKind;
use crate::slot::Slot;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub kind: OutcomeKind,
    /// Non-empty only for [`OutcomeKind::Winner`].
    pub winners: Vec<Slot>,
    /// Indexed by [`Slot::index`].
    pub scores: Vec<i64>,
}
