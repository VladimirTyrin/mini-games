use crate::model::CellPair;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// The order of the two cells is irrelevant.
    RemovePair(CellPair),
    /// Appends a copy of every active value (reading order) after the last non-empty cell.
    Refill,
    /// Rejected when hints are disabled or used up, and while a hint is already shown (it would
    /// show the same hint again).
    RequestHint,
}
