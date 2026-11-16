use crate::cell::Cell;
use crate::merge_source::MergeSource;

/// What happened to a tile in the latest frame: the latest move and the spawn that followed it
/// (before the first move: the two initial spawns). Drives the client's animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileOrigin {
    /// Existed before the latest move and slid from `from` (equal to its cell if it stayed put).
    Slid { from: Cell },
    /// Created by the latest move. `sources[0]` is the tile nearer to the edge the move went
    /// towards. Animate both sources sliding into this tile's cell, then show this tile.
    Merged { sources: [MergeSource; 2] },
    /// Appeared after the latest move (or at the start of the game).
    Spawned,
}
