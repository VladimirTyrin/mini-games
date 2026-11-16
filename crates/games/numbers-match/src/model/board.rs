use crate::COLUMNS;
use crate::model::{Cell, CellIndex, CellPair, CellValue, RowIndex};

/// `cells.len()` is always a multiple of [`COLUMNS`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Board {
    cells: Vec<Cell>,
}

impl Board {
    pub(crate) fn dealt(values: &[CellValue]) -> Self {
        let mut cells: Vec<Cell> = values.iter().copied().map(Cell::Active).collect();
        pad_to_whole_rows(&mut cells);
        Self { cells }
    }

    #[cfg(test)]
    pub(crate) fn from_cells(mut cells: Vec<Cell>) -> Self {
        pad_to_whole_rows(&mut cells);
        Self { cells }
    }

    pub(crate) fn cells(&self) -> &[Cell] {
        &self.cells
    }

    pub(crate) fn rows(&self) -> usize {
        self.cells.len() / COLUMNS
    }

    pub(crate) fn active_count(&self) -> usize {
        self.cells.iter().filter(|cell| cell.is_active()).count()
    }

    fn active_value(&self, index: usize) -> Option<CellValue> {
        self.cells.get(index).and_then(|cell| cell.active_value())
    }

    /// Legacy rule: two distinct active cells whose values pair up, with every cell between them
    /// on a straight line (horizontal, vertical, diagonal) or every cell between them in reading
    /// order inactive.
    pub(crate) fn can_remove_pair(&self, pair: CellPair) -> bool {
        let (first, second) = (pair.first.as_usize(), pair.second.as_usize());
        if first == second {
            return false;
        }
        let (Some(first_value), Some(second_value)) =
            (self.active_value(first), self.active_value(second))
        else {
            return false;
        };
        first_value.pairs_with(second_value)
            && (self.clear_line_between(first, second)
                || self.clear_reading_path_between(first, second))
    }

    fn clear_reading_path_between(&self, first: usize, second: usize) -> bool {
        let (low, high) = (first.min(second), first.max(second));
        self.cells
            .get(low + 1..high)
            .is_some_and(|between| between.iter().all(|cell| !cell.is_active()))
    }

    fn clear_line_between(&self, first: usize, second: usize) -> bool {
        let (first_row, first_column) = ((first / COLUMNS) as isize, (first % COLUMNS) as isize);
        let (second_row, second_column) =
            ((second / COLUMNS) as isize, (second % COLUMNS) as isize);
        let (row_delta, column_delta) = (second_row - first_row, second_column - first_column);
        if row_delta != 0 && column_delta != 0 && row_delta.abs() != column_delta.abs() {
            return false;
        }
        let (row_step, column_step) = (row_delta.signum(), column_delta.signum());
        let (mut row, mut column) = (first_row + row_step, first_column + column_step);
        while (row, column) != (second_row, second_column) {
            // Both ends are on the board, so every cell in between is too.
            if self
                .active_value(row as usize * COLUMNS + column as usize)
                .is_some()
            {
                return false;
            }
            row += row_step;
            column += column_step;
        }
        true
    }

    /// The caller has validated the pair.
    pub(crate) fn remove_pair(&mut self, pair: CellPair) {
        for index in [pair.first, pair.second] {
            if let Some(cell) = self.cells.get_mut(index.as_usize())
                && let Cell::Active(value) = *cell
            {
                *cell = Cell::Removed(value);
            }
        }
    }

    /// Returns the deleted rows as indices *before* deletion, ascending.
    pub(crate) fn delete_inactive_rows(&mut self) -> Vec<RowIndex> {
        let mut deleted = Vec::new();
        let mut kept = Vec::with_capacity(self.cells.len());
        for (row, cells) in self.cells.chunks(COLUMNS).enumerate() {
            if cells.iter().any(|cell| cell.is_active()) {
                kept.extend_from_slice(cells);
            } else {
                deleted.push(RowIndex::from_usize(row));
            }
        }
        if !deleted.is_empty() {
            self.cells = kept;
        }
        deleted
    }

    /// Appends a copy of every active value after the last non-empty cell. Returns where the
    /// copy starts and the copied values.
    pub(crate) fn refill(&mut self) -> (CellIndex, Vec<CellValue>) {
        let values: Vec<CellValue> = self
            .cells
            .iter()
            .filter_map(|cell| cell.active_value())
            .collect();
        let start = self
            .cells
            .iter()
            .rposition(|cell| *cell != Cell::Empty)
            .map_or(0, |last| last + 1);
        if !values.is_empty() {
            // Only empty padding follows `start`.
            self.cells.truncate(start);
            self.cells.extend(values.iter().copied().map(Cell::Active));
            pad_to_whole_rows(&mut self.cells);
        }
        (CellIndex::from_usize(start), values)
    }

    /// The pair legacy's brute-force scan would find first: the lexicographically smallest
    /// removable `(first, second)` with `first < second`. O(cells), no allocation.
    ///
    /// Why scanning neighbours is enough: a pair is removable only if every cell between them on
    /// a line, or in reading order, is inactive. That is exactly "consecutive active cells" on a
    /// column, a diagonal, an anti-diagonal, or in reading order (rows are a special case of
    /// reading order). So walking each such line once and testing neighbouring active cells
    /// visits every removable pair. `fast_finder_matches_brute_force_*` tests check it.
    pub(crate) fn find_pair(&self) -> Option<CellPair> {
        let mut best = None;
        self.scan_line(0..self.cells.len(), &mut best);
        for column in 0..COLUMNS {
            self.scan_line(self.ray_down(0, column, 0), &mut best);
            self.scan_line(self.ray_down(0, column, 1), &mut best);
            self.scan_line(self.ray_down(0, column, -1), &mut best);
        }
        for row in 1..self.rows() {
            self.scan_line(self.ray_down(row, 0, 1), &mut best);
            self.scan_line(self.ray_down(row, COLUMNS - 1, -1), &mut best);
        }
        best
    }

    /// Indices from `(row, column)` moving one row down and `column_step` columns per step.
    fn ray_down(
        &self,
        row: usize,
        column: usize,
        column_step: isize,
    ) -> impl Iterator<Item = usize> + use<> {
        let rows = self.rows();
        (0usize..).map_while(move |step| {
            let current_row = row + step;
            let current_column = column.checked_add_signed(column_step * step as isize)?;
            (current_row < rows && current_column < COLUMNS)
                .then_some(current_row * COLUMNS + current_column)
        })
    }

    /// `line` must yield ascending indices.
    fn scan_line(&self, line: impl Iterator<Item = usize>, best: &mut Option<CellPair>) {
        let mut previous: Option<(usize, CellValue)> = None;
        for index in line {
            let Some(value) = self.active_value(index) else {
                continue;
            };
            if let Some((previous_index, previous_value)) = previous
                && previous_value.pairs_with(value)
            {
                let candidate = CellPair::new(
                    CellIndex::from_usize(previous_index),
                    CellIndex::from_usize(index),
                );
                if best.is_none_or(|current| candidate < current) {
                    *best = Some(candidate);
                }
            }
            previous = Some((index, value));
        }
    }

    /// Legacy O(n³) finder: the reference for property tests.
    #[cfg(test)]
    pub(crate) fn find_pair_brute_force(&self) -> Option<CellPair> {
        let active: Vec<CellIndex> = (0..self.cells.len())
            .filter(|&index| self.active_value(index).is_some())
            .map(CellIndex::from_usize)
            .collect();
        for (position, &first) in active.iter().enumerate() {
            for &second in &active[position + 1..] {
                let pair = CellPair::new(first, second);
                if self.can_remove_pair(pair) {
                    return Some(pair);
                }
            }
        }
        None
    }
}

fn pad_to_whole_rows(cells: &mut Vec<Cell>) {
    cells.resize(cells.len().div_ceil(COLUMNS) * COLUMNS, Cell::Empty);
}
