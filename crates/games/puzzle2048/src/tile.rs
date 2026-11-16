use crate::cell::Cell;
use crate::tile_id::TileId;
use crate::tile_origin::TileOrigin;
use crate::tile_value::TileValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile {
    pub id: TileId,
    pub value: TileValue,
    pub cell: Cell,
    pub origin: TileOrigin,
}
