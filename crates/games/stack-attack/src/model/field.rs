use crate::model::{BoxId, CargoBox, Cell, Column, Row};

/// Occupancy grid plus the box list. Both are only changed together here, and no method ever
/// puts a box on an occupied cell (legacy overwrote cells and left "ghost" boxes, A2).
#[derive(Clone, Debug)]
pub(crate) struct Field {
    width: i32,
    height: i32,
    /// Row-major, row 0 is the floor.
    grid: Vec<Option<BoxId>>,
    /// Sorted by id, so lookups are binary searches and iteration order is canonical.
    boxes: Vec<CargoBox>,
}

impl Field {
    /// `width` and `height` come from validated settings.
    pub(crate) fn new(width: i32, height: i32) -> Self {
        let cells = usize::try_from(width.saturating_mul(height)).unwrap_or(0);
        Self {
            width,
            height,
            grid: vec![None; cells],
            boxes: Vec::new(),
        }
    }

    pub(crate) fn width(&self) -> i32 {
        self.width
    }

    pub(crate) fn height(&self) -> i32 {
        self.height
    }

    pub(crate) fn top_row(&self) -> i32 {
        self.height - 1
    }

    pub(crate) fn contains_column(&self, column: Column) -> bool {
        (0..self.width).contains(&column.get())
    }

    pub(crate) fn contains(&self, cell: Cell) -> bool {
        self.contains_column(cell.column()) && (0..self.height).contains(&cell.y)
    }

    fn grid_index(&self, cell: Cell) -> Option<usize> {
        if self.contains(cell) {
            usize::try_from(cell.y * self.width + cell.x).ok()
        } else {
            None
        }
    }

    pub(crate) fn box_at(&self, cell: Cell) -> Option<BoxId> {
        self.grid_index(cell)
            .and_then(|index| self.grid.get(index).copied().flatten())
    }

    pub(crate) fn is_occupied(&self, cell: Cell) -> bool {
        self.box_at(cell).is_some()
    }

    pub(crate) fn boxes(&self) -> &[CargoBox] {
        &self.boxes
    }

    fn get(&self, id: BoxId) -> Option<&CargoBox> {
        let position = self.boxes.binary_search_by_key(&id, |b| b.id).ok()?;
        self.boxes.get(position)
    }

    fn get_mut(&mut self, id: BoxId) -> Option<&mut CargoBox> {
        let position = self.boxes.binary_search_by_key(&id, |b| b.id).ok()?;
        self.boxes.get_mut(position)
    }

    /// Refuses off-field or occupied cells and duplicate ids.
    pub(crate) fn try_add(&mut self, cargo: CargoBox) -> bool {
        let Some(index) = self.grid_index(cargo.cell) else {
            return false;
        };
        if self.is_occupied(cargo.cell) {
            return false;
        }
        let Err(position) = self.boxes.binary_search_by_key(&cargo.id, |b| b.id) else {
            return false;
        };
        if let Some(slot) = self.grid.get_mut(index) {
            *slot = Some(cargo.id);
            self.boxes.insert(position, cargo);
            true
        } else {
            false
        }
    }

    /// Refuses off-field or occupied targets and unknown ids.
    pub(crate) fn try_move(&mut self, id: BoxId, to: Cell) -> bool {
        let Some(to_index) = self.grid_index(to) else {
            return false;
        };
        if self.is_occupied(to) {
            return false;
        }
        let Some(cargo) = self.get_mut(id) else {
            return false;
        };
        let from = cargo.cell;
        cargo.cell = to;
        let from_index = self.grid_index(from);
        if let Some(cell) = from_index.and_then(|index| self.grid.get_mut(index)) {
            *cell = None;
        }
        if let Some(cell) = self.grid.get_mut(to_index) {
            *cell = Some(id);
        }
        true
    }

    fn set_falling(&mut self, id: BoxId, falling: bool) {
        if let Some(cargo) = self.get_mut(id) {
            cargo.falling = falling;
        }
    }

    fn cells_bottom_up(&self) -> impl Iterator<Item = Cell> + use<> {
        let width = self.width;
        (0..self.height).flat_map(move |y| (0..width).map(move |x| Cell::new(x, y)))
    }

    /// Every unsupported box falls one cell; returns the boxes that landed this tick.
    ///
    /// Support is re-evaluated for every box every tick, not only for boxes legacy marked as
    /// falling, so pushed boxes (A1) and boxes left hanging by a line clear (A9) fall too.
    /// Scanning bottom-up means a box only ever sees resting boxes or empty cells below it (a
    /// lower box has either landed or already moved away), so stacks fall together.
    pub(crate) fn apply_gravity(&mut self) -> Vec<(BoxId, Cell)> {
        let mut landed = Vec::new();
        for cell in self.cells_bottom_up() {
            let Some(id) = self.box_at(cell) else {
                continue;
            };
            let supported = cell.y == 0 || self.is_occupied(cell.below());
            if supported {
                if self.get(id).is_some_and(|b| b.falling) {
                    self.set_falling(id, false);
                    landed.push((id, cell));
                }
            } else if self.try_move(id, cell.below()) {
                self.set_falling(id, true);
            }
        }
        landed
    }

    /// Removes full rows (falling boxes count, as in legacy) and shifts everything above each
    /// cleared row down by one. Returns the cleared rows, bottom first.
    pub(crate) fn clear_full_rows(&mut self) -> Vec<Row> {
        let full: Vec<i32> = (0..self.height)
            .filter(|&y| (0..self.width).all(|x| self.is_occupied(Cell::new(x, y))))
            .collect();
        for &row in full.iter().rev() {
            self.boxes.retain(|b| b.cell.y != row);
            for x in 0..self.width {
                if let Some(cell) = self
                    .grid_index(Cell::new(x, row))
                    .and_then(|index| self.grid.get_mut(index))
                {
                    *cell = None;
                }
            }
            for y in row + 1..self.height {
                for x in 0..self.width {
                    let cell = Cell::new(x, y);
                    if let Some(id) = self.box_at(cell) {
                        self.try_move(id, cell.below());
                    }
                }
            }
        }
        if !full.is_empty() {
            self.refresh_support();
        }
        full.into_iter().map(Row::new).collect()
    }

    /// A9: after a clear the shifted boxes may rest on nothing; mark them falling right away so
    /// the ceiling check and the view agree with what gravity does next tick.
    fn refresh_support(&mut self) {
        for cell in self.cells_bottom_up() {
            let Some(id) = self.box_at(cell) else {
                continue;
            };
            let supported = cell.y == 0
                || self
                    .box_at(cell.below())
                    .and_then(|below| self.get(below))
                    .is_some_and(|below| !below.falling);
            self.set_falling(id, !supported);
        }
    }

    pub(crate) fn has_resting_box_in_top_row(&self) -> bool {
        let top = self.top_row();
        self.boxes.iter().any(|b| b.cell.y == top && !b.falling)
    }

    #[cfg(test)]
    pub(crate) fn check_consistency(&self) -> Result<(), String> {
        let mut seen = 0usize;
        for cargo in &self.boxes {
            if self.box_at(cargo.cell) != Some(cargo.id) {
                return Err(format!("box {:?} is not in the grid at its cell", cargo));
            }
            seen += 1;
        }
        let in_grid = self.grid.iter().filter(|c| c.is_some()).count();
        if in_grid != seen {
            return Err(format!(
                "{in_grid} grid cells occupied, {seen} boxes listed"
            ));
        }
        if !self.boxes.windows(2).all(|w| w[0].id < w[1].id) {
            return Err("box list is not strictly sorted by id".into());
        }
        Ok(())
    }
}
