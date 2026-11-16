use crate::model::{CellContent, Field, Point};

/// What occupies each cell, kept in sync with snake bodies and food so collision and spawn
/// checks are O(1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Grid {
    field: Field,
    cells: Vec<CellContent>,
}

impl Grid {
    pub(crate) fn new(field: Field) -> Self {
        Self {
            field,
            cells: vec![CellContent::Empty; field.cell_count()],
        }
    }

    /// `None` outside the field.
    pub(crate) fn get(&self, point: Point) -> Option<CellContent> {
        self.field
            .index_of(point)
            .and_then(|index| self.cells.get(index))
            .copied()
    }

    pub(crate) fn set(&mut self, point: Point, content: CellContent) {
        if let Some(cell) = self
            .field
            .index_of(point)
            .and_then(|index| self.cells.get_mut(index))
        {
            *cell = content;
        }
    }

    pub(crate) fn is_empty_cell(&self, point: Point) -> bool {
        self.get(point) == Some(CellContent::Empty)
    }

    pub(crate) fn empty_cell_count(&self) -> usize {
        self.cells
            .iter()
            .filter(|&&cell| cell == CellContent::Empty)
            .count()
    }

    /// The `nth` empty cell in row-major order.
    pub(crate) fn nth_empty_cell(&self, nth: usize) -> Option<Point> {
        let index = self
            .cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| **cell == CellContent::Empty)
            .nth(nth)
            .map(|(index, _)| index)?;
        self.field.point_at(index)
    }
}
