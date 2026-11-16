/// Cosmetic box texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PatternId(u8);

impl PatternId {
    pub const COUNT: u8 = 8;

    pub const fn new(value: u8) -> Option<Self> {
        if value < Self::COUNT {
            Some(Self(value))
        } else {
            None
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}
