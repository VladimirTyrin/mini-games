use crate::model::{CellIndex, CellPair, CellValue, RowIndex};

/// What the last accepted command did, so the UI can animate it. Replaces legacy's event queue
/// (which was re-sent forever): this is simply overwritten by the next command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LastAction {
    PairRemoved {
        /// Indices in the board *before* rows were deleted.
        pair: CellPair,
        /// Rows left without active cells and deleted, as indices in the board *before*
        /// deletion, ascending (at most two).
        deleted_rows: Vec<RowIndex>,
    },
    Refilled {
        /// Index of the first appended cell.
        start: CellIndex,
        values: Vec<CellValue>,
    },
    HintShown,
}
