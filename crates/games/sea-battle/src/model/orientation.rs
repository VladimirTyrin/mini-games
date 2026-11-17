use crate::model::Cell;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Orientation {
    /// From the bow to the right.
    Horizontal,
    /// From the bow down.
    Vertical,
}

impl Orientation {
    /// The cell `steps` along this orientation from `from`, if still on the board.
    pub(crate) fn advance(self, from: Cell, steps: i8) -> Option<Cell> {
        match self {
            Orientation::Horizontal => from.offset(steps, 0),
            Orientation::Vertical => from.offset(0, steps),
        }
    }

    pub(crate) const fn hash_code(self) -> u8 {
        match self {
            Orientation::Horizontal => 0,
            Orientation::Vertical => 1,
        }
    }
}
