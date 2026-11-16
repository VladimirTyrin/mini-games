use std::fmt;

/// Seat index at the table. Seats follow lobby join order (slot 0 is the host) and never change
/// during a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Slot(u8);

impl Slot {
    pub const fn new(index: u8) -> Self {
        Self(index)
    }

    pub const fn get(self) -> u8 {
        self.0
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// All seats of a table with `seats` players.
    pub fn all(seats: usize) -> impl Iterator<Item = Slot> {
        (0..seats.min(usize::from(u8::MAX) + 1)).map(|i| Slot(i as u8))
    }
}

impl TryFrom<usize> for Slot {
    type Error = std::num::TryFromIntError;

    fn try_from(index: usize) -> Result<Self, Self::Error> {
        u8::try_from(index).map(Slot)
    }
}

impl fmt::Display for Slot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}
