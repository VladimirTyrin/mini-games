use mg_core::{SettingRange, SetupError, StateHasher};

use crate::model::{Cell, Direction, Score, Tile, TileId, TileValue};
use crate::rules::SlideOutcome;
use crate::settings::Settings;
use crate::view::{MergeSource, TileOrigin};

#[derive(Clone, Debug)]
pub(crate) struct Board {
    width: u8,
    height: u8,
    /// Row-major; every tile's `cell` matches its slot.
    slots: Vec<Option<Tile>>,
}

impl Board {
    /// Validates the size before allocating.
    pub(crate) fn empty(width: u32, height: u32) -> Result<Self, SetupError> {
        let width = Self::checked_size(Settings::WIDTH, width)?;
        let height = Self::checked_size(Settings::HEIGHT, height)?;
        Ok(Self {
            width,
            height,
            slots: vec![None; usize::from(width) * usize::from(height)],
        })
    }

    fn checked_size(range: SettingRange, value: u32) -> Result<u8, SetupError> {
        range.check(value)?;
        u8::try_from(value).map_err(|error| SetupError::setting(range.field, error.to_string()))
    }

    pub(crate) fn width(&self) -> u8 {
        self.width
    }

    pub(crate) fn height(&self) -> u8 {
        self.height
    }

    fn slot_index(&self, cell: Cell) -> Option<usize> {
        (cell.row < self.height && cell.col < self.width)
            .then(|| usize::from(cell.row) * usize::from(self.width) + usize::from(cell.col))
    }

    pub(crate) fn contains(&self, cell: Cell) -> bool {
        self.slot_index(cell).is_some()
    }

    pub(crate) fn tile(&self, cell: Cell) -> Option<&Tile> {
        self.slot_index(cell)
            .and_then(|index| self.slots.get(index))
            .and_then(Option::as_ref)
    }

    /// Row-major; the view exposes this order.
    pub(crate) fn tiles(&self) -> impl Iterator<Item = &Tile> {
        self.slots.iter().flatten()
    }

    /// Row-major; `roll_world` indexes into this list, so the order is part of determinism.
    pub(crate) fn empty_cells(&self) -> Vec<Cell> {
        let width = self.width;
        (0..self.height)
            .flat_map(|row| (0..width).map(move |col| Cell::new(row, col)))
            .filter(|&cell| self.tile(cell).is_none())
            .collect()
    }

    /// Overwrites whatever is at `tile.cell`; callers check that the cell is free and on the board.
    pub(crate) fn put(&mut self, tile: Tile) {
        if let Some(slot) = self
            .slot_index(tile.cell)
            .and_then(|index| self.slots.get_mut(index))
        {
            *slot = Some(tile);
        }
    }

    pub(crate) fn highest_tile(&self) -> Option<TileValue> {
        self.tiles().map(|tile| tile.value).max()
    }

    fn line_count(&self, direction: Direction) -> u8 {
        match direction {
            Direction::Left | Direction::Right => self.height,
            Direction::Up | Direction::Down => self.width,
        }
    }

    /// Cells of one row or column, starting at the edge the move goes towards.
    fn line(&self, direction: Direction, line: u8) -> Vec<Cell> {
        match direction {
            Direction::Left => (0..self.width).map(|col| Cell::new(line, col)).collect(),
            Direction::Right => (0..self.width)
                .rev()
                .map(|col| Cell::new(line, col))
                .collect(),
            Direction::Up => (0..self.height).map(|row| Cell::new(row, line)).collect(),
            Direction::Down => (0..self.height)
                .rev()
                .map(|row| Cell::new(row, line))
                .collect(),
        }
    }

    /// Classic 2048 slide: each line is compacted towards the leading edge and equal neighbours
    /// merge pairwise from that edge (`[2, 2, 2]` → `[4, 2]`); a merged tile does not merge again
    /// in the same move. Merged tiles get ids starting at `next_tile_id`, in line order.
    pub(crate) fn slide(&self, direction: Direction, next_tile_id: TileId) -> SlideOutcome {
        let mut board = Board {
            width: self.width,
            height: self.height,
            slots: vec![None; self.slots.len()],
        };
        let mut next_tile_id = next_tile_id;
        let mut gained = Score::ZERO;
        let mut changed = false;
        for line in 0..self.line_count(direction) {
            let cells = self.line(direction, line);
            let mut tiles = cells.iter().filter_map(|&cell| self.tile(cell)).peekable();
            let mut destinations = cells.iter().copied();
            while let Some(first) = tiles.next() {
                let Some(destination) = destinations.next() else {
                    break;
                };
                let tile = match tiles.next_if(|tile| tile.value == first.value) {
                    Some(second) => {
                        let id = next_tile_id;
                        next_tile_id = next_tile_id.next();
                        let value = first.value.doubled();
                        gained = gained.plus_merge(value);
                        changed = true;
                        Tile {
                            id,
                            value,
                            cell: destination,
                            origin: TileOrigin::Merged {
                                sources: [
                                    MergeSource {
                                        id: first.id,
                                        from: first.cell,
                                    },
                                    MergeSource {
                                        id: second.id,
                                        from: second.cell,
                                    },
                                ],
                            },
                        }
                    }
                    None => {
                        changed |= first.cell != destination;
                        Tile {
                            id: first.id,
                            value: first.value,
                            cell: destination,
                            origin: TileOrigin::Slid { from: first.cell },
                        }
                    }
                };
                board.put(tile);
            }
        }
        SlideOutcome {
            board,
            gained,
            next_tile_id,
            changed,
        }
    }

    pub(crate) fn has_moves(&self) -> bool {
        Direction::ALL
            .iter()
            .any(|&direction| self.slide(direction, TileId::FIRST).changed)
    }

    pub(crate) fn hash(&self, hasher: &mut StateHasher) {
        hasher.u8(self.width);
        hasher.u8(self.height);
        for slot in &self.slots {
            let Some(tile) = slot else {
                hasher.u8(0);
                continue;
            };
            hasher.u8(1);
            hasher.u32(tile.id.get());
            hasher.u32(tile.value.get());
            hash_cell(hasher, tile.cell);
            match tile.origin {
                TileOrigin::Slid { from } => {
                    hasher.u8(0);
                    hash_cell(hasher, from);
                }
                TileOrigin::Merged { sources } => {
                    hasher.u8(1);
                    for source in sources {
                        hasher.u32(source.id.get());
                        hash_cell(hasher, source.from);
                    }
                }
                TileOrigin::Spawned => hasher.u8(2),
            }
        }
    }
}

fn hash_cell(hasher: &mut StateHasher, cell: Cell) {
    hasher.u8(cell.row);
    hasher.u8(cell.col);
}
