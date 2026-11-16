use crate::model::{Cell, TileId, TileValue};
use crate::view::TileOrigin;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile {
    pub id: TileId,
    pub value: TileValue,
    pub cell: Cell,
    pub origin: TileOrigin,
}
