use crate::model::{Cell, Direction, Score, Tile, TileValue};
use crate::rules::Phase;
use crate::settings::Target;

/// Render model; identical for the player and spectators.
///
/// Animating a frame needs only the new view: slide every [`TileOrigin::Slid`] tile from its
/// `from` cell, slide both [`TileOrigin::Merged`] sources into the merged tile's cell, then pop
/// merged tiles and fade in [`TileOrigin::Spawned`] ones.
///
/// [`TileOrigin::Slid`]: crate::TileOrigin::Slid
/// [`TileOrigin::Merged`]: crate::TileOrigin::Merged
/// [`TileOrigin::Spawned`]: crate::TileOrigin::Spawned
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub width: u8,
    pub height: u8,
    pub target: Target,
    /// Row-major order.
    pub tiles: Vec<Tile>,
    pub score: Score,
    pub moves: u32,
    /// `None` only before the initial spawns.
    pub highest_tile: Option<TileValue>,
    pub last_move: Option<Direction>,
    pub phase: Phase,
    /// Moves the game accepts right now, in [`Direction::ALL`] order: lets UIs ignore no-op
    /// swipes without a round trip. Empty while a spawn is due and once the game is over.
    pub movable_directions: Vec<Direction>,
}

impl View {
    pub fn tile_at(&self, cell: Cell) -> Option<&Tile> {
        self.tiles.iter().find(|tile| tile.cell == cell)
    }

    /// Row-major tile values with 0 for empty cells (the legacy wire layout).
    pub fn cell_values(&self) -> Vec<u32> {
        let width = usize::from(self.width);
        let mut values = vec![0; width * usize::from(self.height)];
        for tile in &self.tiles {
            let index = usize::from(tile.cell.row) * width + usize::from(tile.cell.col);
            if let Some(value) = values.get_mut(index) {
                *value = tile.value.get();
            }
        }
        values
    }
}
