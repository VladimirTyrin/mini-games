use mg_core::StateHasher;

/// One square of a 10×10 sea: `x` runs left to right (columns A–J), `y` top to bottom (rows
/// 1–10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell {
    x: u8,
    y: u8,
}

impl Cell {
    pub const SIDE: u8 = 10;
    pub const COUNT: usize = 100;

    pub const fn new(x: u8, y: u8) -> Option<Self> {
        if x < Self::SIDE && y < Self::SIDE {
            Some(Self { x, y })
        } else {
            None
        }
    }

    pub const fn x(self) -> u8 {
        self.x
    }

    pub const fn y(self) -> u8 {
        self.y
    }

    /// Row-major, `0..COUNT`.
    pub const fn index(self) -> usize {
        self.y as usize * Self::SIDE as usize + self.x as usize
    }

    /// Row by row, the order every per-cell list follows.
    pub fn all() -> impl Iterator<Item = Cell> {
        (0..Self::SIDE).flat_map(|y| (0..Self::SIDE).map(move |x| Cell { x, y }))
    }

    /// `(dx, dy)` away, if still on the board.
    pub fn offset(self, dx: i8, dy: i8) -> Option<Cell> {
        let x = self.x.checked_add_signed(dx)?;
        let y = self.y.checked_add_signed(dy)?;
        Cell::new(x, y)
    }

    /// The up to 4 cells sharing a side.
    pub fn sides(self) -> impl Iterator<Item = Cell> {
        [(0, -1), (1, 0), (0, 1), (-1, 0)]
            .into_iter()
            .filter_map(move |(dx, dy)| self.offset(dx, dy))
    }

    /// The up to 8 cells sharing a side or a corner.
    pub fn around(self) -> impl Iterator<Item = Cell> {
        (-1..=1)
            .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
            .filter(|offset| *offset != (0, 0))
            .filter_map(move |(dx, dy)| self.offset(dx, dy))
    }

    pub(crate) fn hash(self, hasher: &mut StateHasher) {
        hasher.u8(self.x);
        hasher.u8(self.y);
    }
}
