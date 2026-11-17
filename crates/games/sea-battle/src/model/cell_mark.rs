/// What everyone knows about one cell of a seat's sea.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CellMark {
    #[default]
    Unknown,
    /// Shot at: water.
    Miss,
    /// Shot at: a ship, not (yet) sunk.
    Hit,
    /// Part of a sunk ship.
    Sunk,
    /// Next to a sunk ship, so it must be water: nobody needs to shoot there.
    Cleared,
}

impl CellMark {
    pub(crate) const fn hash_code(self) -> u8 {
        match self {
            CellMark::Unknown => 0,
            CellMark::Miss => 1,
            CellMark::Hit => 2,
            CellMark::Sunk => 3,
            CellMark::Cleared => 4,
        }
    }

    /// Whether a shot there could tell anything new.
    pub const fn is_open(self) -> bool {
        matches!(self, CellMark::Unknown)
    }
}
