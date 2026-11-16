use crate::cell::Cell;
use crate::spawn_value::SpawnValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldEvent {
    /// Two at the start of the game, then one after every accepted move. The cell must be empty.
    TileSpawned { cell: Cell, value: SpawnValue },
}
