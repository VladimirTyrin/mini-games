/// A digit `1..=9`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellValue(u8);

impl CellValue {
    pub const MIN: u8 = 1;
    pub const MAX: u8 = 9;

    pub const fn new(value: u8) -> Option<Self> {
        if value >= Self::MIN && value <= Self::MAX {
            Some(Self(value))
        } else {
            None
        }
    }

    /// For values that are in range by construction (RNG output); clamps instead of panicking.
    pub(crate) const fn clamped(value: u8) -> Self {
        if value < Self::MIN {
            Self(Self::MIN)
        } else if value > Self::MAX {
            Self(Self::MAX)
        } else {
            Self(value)
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }

    /// The pairing rule: equal, or summing to 10.
    pub const fn pairs_with(self, other: Self) -> bool {
        self.0 == other.0 || self.0 + other.0 == 10
    }
}
