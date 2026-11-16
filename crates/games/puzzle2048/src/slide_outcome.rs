use crate::board::Board;
use crate::score::Score;
use crate::tile_id::TileId;

/// A slid board, not yet committed to the game.
#[derive(Debug)]
pub(crate) struct SlideOutcome {
    pub(crate) board: Board,
    pub(crate) gained: Score,
    pub(crate) next_tile_id: TileId,
    pub(crate) changed: bool,
}
