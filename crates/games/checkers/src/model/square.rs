use super::Direction;

/// A playable (dark) square of the 8×8 board. `(0, 0)` is a1, white's left corner; the
/// playable squares are those with `x + y` even. Valid by construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Square {
    x: u8,
    y: u8,
}

impl Square {
    pub const SIZE: u8 = 8;

    pub fn new(x: u8, y: u8) -> Option<Self> {
        (x < Self::SIZE && y < Self::SIZE && (x + y).is_multiple_of(2)).then_some(Self { x, y })
    }

    /// From the wire: `y * 8 + x`.
    pub fn from_index(index: u32) -> Option<Self> {
        let index = u8::try_from(index).ok()?;
        Self::new(index % Self::SIZE, index / Self::SIZE)
    }

    pub fn index(self) -> u32 {
        u32::from(self.y) * u32::from(Self::SIZE) + u32::from(self.x)
    }

    pub fn x(self) -> u8 {
        self.x
    }

    pub fn y(self) -> u8 {
        self.y
    }

    /// The next square in `direction`, if still on the board (it is dark again).
    pub(crate) fn step(self, direction: Direction) -> Option<Self> {
        let x = self.x.checked_add_signed(direction.dx())?;
        let y = self.y.checked_add_signed(direction.dy())?;
        Self::new(x, y)
    }

    /// Every playable square, a1 first, row by row.
    pub(crate) fn all() -> impl Iterator<Item = Square> {
        (0..Self::SIZE).flat_map(|y| (0..Self::SIZE).filter_map(move |x| Self::new(x, y)))
    }

    /// Index into a 64-cell array.
    pub(crate) fn cell(self) -> usize {
        usize::from(self.y) * usize::from(Self::SIZE) + usize::from(self.x)
    }
}
