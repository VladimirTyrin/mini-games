/// A board coordinate: `x` is the column, `y` the row, `(0, 0)` is the top-left corner. Comes
/// from untrusted input, so it may lie outside the board; the game checks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

impl Point {
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}
