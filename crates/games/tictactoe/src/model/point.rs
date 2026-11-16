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

    /// `None` when the result would leave the non-negative quadrant.
    pub(crate) fn offset(self, dx: i32, dy: i32) -> Option<Point> {
        Some(Point::new(
            self.x.checked_add_signed(dx)?,
            self.y.checked_add_signed(dy)?,
        ))
    }
}
