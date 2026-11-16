use crate::cell_value::CellValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    /// Padding after the last value so the board is whole rows. Refills write over it.
    Empty,
    Active(CellValue),
    /// Matched: shown greyed out, no longer blocks paths.
    Removed(CellValue),
}

impl Cell {
    pub fn is_active(self) -> bool {
        matches!(self, Cell::Active(_))
    }

    pub fn active_value(self) -> Option<CellValue> {
        match self {
            Cell::Active(value) => Some(value),
            Cell::Empty | Cell::Removed(_) => None,
        }
    }
}
