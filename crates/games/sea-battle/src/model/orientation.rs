#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Orientation {
    /// From the bow to the right.
    Horizontal,
    /// From the bow down.
    Vertical,
}

impl Orientation {
    pub(crate) const fn step(self) -> (i8, i8) {
        match self {
            Orientation::Horizontal => (1, 0),
            Orientation::Vertical => (0, 1),
        }
    }

    pub(crate) const fn hash_code(self) -> u8 {
        match self {
            Orientation::Horizontal => 0,
            Orientation::Vertical => 1,
        }
    }
}
