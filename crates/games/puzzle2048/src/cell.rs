/// A board position; `(0, 0)` is the top-left corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell {
    pub row: u8,
    pub col: u8,
}

impl Cell {
    pub const fn new(row: u8, col: u8) -> Self {
        Self { row, col }
    }
}
