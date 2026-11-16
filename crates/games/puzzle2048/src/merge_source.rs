use crate::cell::Cell;
use crate::tile_id::TileId;

/// One of the two tiles consumed by a merge; it no longer exists after the move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MergeSource {
    pub id: TileId,
    pub from: Cell,
}
