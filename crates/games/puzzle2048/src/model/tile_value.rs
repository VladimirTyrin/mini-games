/// A tile's number: always a power of two, at least 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileValue(u32);

impl TileValue {
    pub const TWO: TileValue = TileValue(2);
    pub const FOUR: TileValue = TileValue(4);

    /// `None` unless `value` is a power of two of at least 2.
    pub fn new(value: u32) -> Option<Self> {
        (value >= 2 && value.is_power_of_two()).then_some(Self(value))
    }

    pub fn get(self) -> u32 {
        self.0
    }

    /// Cannot saturate in play: the game ends once a tile reaches the target, which is at most
    /// `Settings::MAX_TARGET`.
    pub(crate) fn doubled(self) -> TileValue {
        TileValue(self.0.saturating_mul(2))
    }
}
