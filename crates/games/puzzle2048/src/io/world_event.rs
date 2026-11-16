use crate::model::{Cell, SpawnValue};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldEvent {
    /// Two at the start of the game, then one after every accepted move. The cell must be empty.
    TileSpawned { cell: Cell, value: SpawnValue },
}
