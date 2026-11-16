use crate::cell_value::CellValue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldEvent {
    /// The initial deal, sent once in the first frame: exactly [`crate::INITIAL_CELLS`] values in
    /// reading order, no two horizontally adjacent ones equal (checked by `apply_world`).
    BoardDealt { values: Vec<CellValue> },
}
