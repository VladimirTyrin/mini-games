use crate::tile_value::TileValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnValue {
    Two,
    Four,
}

impl SpawnValue {
    /// A spawn is a 4 with probability `FOUR_CHANCE.0 / FOUR_CHANCE.1`.
    pub const FOUR_CHANCE: (u32, u32) = (1, 10);

    pub fn value(self) -> TileValue {
        match self {
            SpawnValue::Two => TileValue::TWO,
            SpawnValue::Four => TileValue::FOUR,
        }
    }
}
