use super::types::{Cell, Position, FIELD_WIDTH, INITIAL_CELLS};
use crate::games::session_rng::SessionRng;

#[derive(Clone, Debug)]
pub struct Board {
    cells: Vec<Cell>,
    row_count: usize,
}

impl Board {
    pub fn new(rng: &mut SessionRng) -> Self {
        let row_count = INITIAL_CELLS.div_ceil(FIELD_WIDTH);
        let total_cells = row_count * FIELD_WIDTH;
        let mut cells = vec![Cell::empty(); total_cells];

        for i in 0..INITIAL_CELLS {
            let col = i % FIELD_WIDTH;
            let prev_value = if col > 0 { Some(cells[i - 1].value) } else { None };

            loop {
                let value = rng.random_range(1..=9);
                if prev_value != Some(value) {
                    cells[i].value = value;
                    cells[i].removed = false;
                    break;
                }
            }
        }

        Self { cells, row_count }
    }

    #[cfg(test)]
    pub fn from_values(values: &[u8]) -> Self {
        let row_count = values.len().div_ceil(FIELD_WIDTH);
        let total_cells = row_count * FIELD_WIDTH;
        let mut cells = vec![Cell::empty(); total_cells];

        for (i, &value) in values.iter().enumerate() {
            cells[i] = Cell::new(value);
        }

        Self { cells, row_count }
    }

    pub fn row_count(&self) -> usize {
        self.row_count
    }

    pub fn get(&self, pos: Position) -> Option<&Cell> {
        if pos.col >= FIELD_WIDTH || pos.row >= self.row_count {
            return None;
        }
        self.cells.get(pos.to_index())
    }

    pub fn get_mut(&mut self, pos: Position) -> Option<&mut Cell> {
        if pos.col >= FIELD_WIDTH || pos.row >= self.row_count {
            return None;
        }
        let index = pos.to_index();
        self.cells.get_mut(index)
    }

    pub fn can_remove_pair(&self, pos1: Position, pos2: Position) -> bool {
        if pos1 == pos2 {
            return false;
        }

        let cell1 = match self.get(pos1) {
            Some(c) if c.is_active() => c,
            _ => return false,
        };

        let cell2 = match self.get(pos2) {
            Some(c) if c.is_active() => c,
            _ => return false,
        };

        let values_match = cell1.value == cell2.value || cell1.value + cell2.value == 10;
        if !values_match {
            return false;
        }

        self.has_line_of_sight(pos1, pos2) || self.has_sequential_path(pos1, pos2)
    }

    fn has_sequential_path(&self, pos1: Position, pos2: Position) -> bool {
        let idx1 = pos1.to_index();
        let idx2 = pos2.to_index();
        let (start, end) = if idx1 < idx2 {
            (idx1, idx2)
        } else {
            (idx2, idx1)
        };

        for i in (start + 1)..end {
            if let Some(cell) = self.cells.get(i)
                && cell.is_active()
            {
                return false;
            }
        }
        true
    }

    fn has_line_of_sight(&self, pos1: Position, pos2: Position) -> bool {
        let row_diff = pos2.row as i32 - pos1.row as i32;
        let col_diff = pos2.col as i32 - pos1.col as i32;

        let is_horizontal = row_diff == 0;
        let is_vertical = col_diff == 0;
        let is_diagonal = row_diff.abs() == col_diff.abs();

        if !is_horizontal && !is_vertical && !is_diagonal {
            return false;
        }

        let row_step = row_diff.signum();
        let col_step = col_diff.signum();

        let mut current_row = pos1.row as i32 + row_step;
        let mut current_col = pos1.col as i32 + col_step;

        while (current_row, current_col) != (pos2.row as i32, pos2.col as i32) {
            let pos = Position::new(current_row as usize, current_col as usize);
            if let Some(cell) = self.get(pos)
                && cell.is_active()
            {
                return false;
            }
            current_row += row_step;
            current_col += col_step;
        }

        true
    }

    pub fn active_cells(&self) -> impl Iterator<Item = (Position, &Cell)> {
        self.cells
            .iter()
            .enumerate()
            .filter(|(i, _)| *i < self.row_count * FIELD_WIDTH)
            .filter(|(_, cell)| cell.is_active())
            .map(|(i, cell)| (Position::from_index(i), cell))
    }

    pub fn active_cell_count(&self) -> usize {
        self.active_cells().count()
    }

    pub fn remove_empty_rows(&mut self) -> Vec<usize> {
        let mut removed_rows = Vec::new();

        let mut row = 0;
        while row < self.row_count {
            let row_start = row * FIELD_WIDTH;
            let row_end = row_start + FIELD_WIDTH;
            let is_empty = self.cells[row_start..row_end]
                .iter()
                .all(|cell| !cell.is_active());

            if is_empty {
                removed_rows.push(row);
                self.cells.drain(row_start..row_end);
                self.row_count -= 1;
            } else {
                row += 1;
            }
        }

        removed_rows
    }

    pub fn refill(&mut self) -> Vec<u8> {
        let active_values: Vec<u8> = self.active_cells().map(|(_, cell)| cell.value).collect();

        if active_values.is_empty() {
            return Vec::new();
        }

        let last_occupied_index = self
            .cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| cell.value > 0)
            .map(|(i, _)| i)
            .next_back()
            .unwrap_or(0);

        let mut write_index = last_occupied_index + 1;

        for &value in &active_values {
            if write_index >= self.cells.len() {
                self.cells.push(Cell::new(value));
                if write_index % FIELD_WIDTH == 0 || write_index >= self.row_count * FIELD_WIDTH {
                    self.row_count = self.cells.len().div_ceil(FIELD_WIDTH);
                }
            } else {
                self.cells[write_index] = Cell::new(value);
            }
            write_index += 1;
        }

        let total_needed = write_index.div_ceil(FIELD_WIDTH) * FIELD_WIDTH;
        while self.cells.len() < total_needed {
            self.cells.push(Cell::empty());
        }
        self.row_count = total_needed / FIELD_WIDTH;

        active_values
    }

    pub fn find_any_valid_pair(&self) -> Option<(Position, Position)> {
        let active: Vec<(Position, &Cell)> = self.active_cells().collect();

        for i in 0..active.len() {
            for j in (i + 1)..active.len() {
                let (pos1, _) = active[i];
                let (pos2, _) = active[j];
                if self.can_remove_pair(pos1, pos2) {
                    return Some((pos1, pos2));
                }
            }
        }

        None
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells[..self.row_count * FIELD_WIDTH]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::session_rng::SessionRng;

    #[test]
    fn test_board_from_values_creates_correct_layout() {
        let values: Vec<u8> = (0..42).map(|i| (i % 9) + 1).collect();
        let board = Board::from_values(&values);

        assert_eq!(board.row_count(), 5);
        assert_eq!(board.get(Position::new(0, 0)).unwrap().value, 1);
        assert_eq!(board.get(Position::new(0, 8)).unwrap().value, 9);
    }

    #[test]
    fn test_can_remove_pair_equal_values_horizontal() {
        let board = Board::from_values(&[5, 0, 0, 5, 0, 0, 0, 0, 0]);

        assert!(board.can_remove_pair(Position::new(0, 0), Position::new(0, 3)));
    }

    #[test]
    fn test_can_remove_pair_sum_ten_horizontal() {
        let board = Board::from_values(&[3, 0, 7, 0, 0, 0, 0, 0, 0]);

        assert!(board.can_remove_pair(Position::new(0, 0), Position::new(0, 2)));
    }

    #[test]
    fn test_can_remove_pair_blocked_by_other_cell() {
        let board = Board::from_values(&[5, 1, 5, 0, 0, 0, 0, 0, 0]);

        assert!(!board.can_remove_pair(Position::new(0, 0), Position::new(0, 2)));
    }

    #[test]
    fn test_can_remove_pair_diagonal() {
        #[rustfmt::skip]
        let board = Board::from_values(&[
            5, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 5, 0, 0, 0, 0, 0, 0,
        ]);

        assert!(board.can_remove_pair(Position::new(0, 0), Position::new(2, 2)));
    }

    #[test]
    fn test_can_remove_pair_diagonal_blocked() {
        #[rustfmt::skip]
        let board = Board::from_values(&[
            5, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 5, 0, 0, 0, 0, 0, 0,
        ]);

        assert!(!board.can_remove_pair(Position::new(0, 0), Position::new(2, 2)));
    }

    #[test]
    fn test_can_remove_pair_vertical() {
        #[rustfmt::skip]
        let board = Board::from_values(&[
            5, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
            5, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        assert!(board.can_remove_pair(Position::new(0, 0), Position::new(2, 0)));
    }

    #[test]
    fn test_can_remove_pair_sequential_path() {
        #[rustfmt::skip]
        let board = Board::from_values(&[
            5, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 5, 0, 0, 0, 0, 0, 0,
        ]);

        assert!(board.can_remove_pair(Position::new(0, 0), Position::new(1, 2)));
    }

    #[test]
    fn test_can_remove_pair_sequential_path_blocked() {
        #[rustfmt::skip]
        let board = Board::from_values(&[
            5, 0, 0, 0, 1, 0, 0, 0, 0,
            0, 0, 5, 0, 0, 0, 0, 0, 0,
        ]);

        assert!(!board.can_remove_pair(Position::new(0, 0), Position::new(1, 2)));
    }

    #[test]
    fn test_can_remove_pair_same_cell() {
        let board = Board::from_values(&[5, 0, 0, 0, 0, 0, 0, 0, 0]);

        assert!(!board.can_remove_pair(Position::new(0, 0), Position::new(0, 0)));
    }

    #[test]
    fn test_can_remove_pair_different_values_not_sum_ten() {
        let board = Board::from_values(&[3, 5, 0, 0, 0, 0, 0, 0, 0]);

        assert!(!board.can_remove_pair(Position::new(0, 0), Position::new(0, 1)));
    }

    #[test]
    fn test_remove_empty_rows_deletes_empty_row() {
        #[rustfmt::skip]
        let mut board = Board::from_values(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
            1, 2, 3, 4, 5, 6, 7, 8, 9,
        ]);

        let removed = board.remove_empty_rows();

        assert_eq!(removed, vec![1]);
        assert_eq!(board.row_count(), 2);
    }

    #[test]
    fn test_refill_copies_active_cells() {
        #[rustfmt::skip]
        let mut board = Board::from_values(&[
            1, 0, 2, 0, 0, 0, 0, 0, 0,
            3, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 4, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let added = board.refill();

        assert_eq!(added, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_refill_example_from_task() {
        #[rustfmt::skip]
        let mut board = Board::from_values(&[
            1, 0, 2, 0, 0, 0, 0, 0, 0,
            3, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 4, 0, 0, 0, 0, 0, 0, 0,
        ]);

        board.refill();

        assert_eq!(board.get(Position::new(2, 2)).unwrap().value, 1);
        assert_eq!(board.get(Position::new(2, 3)).unwrap().value, 2);
        assert_eq!(board.get(Position::new(2, 4)).unwrap().value, 3);
        assert_eq!(board.get(Position::new(2, 5)).unwrap().value, 4);
    }

    #[test]
    fn test_refill_writes_after_removed_cells() {
        #[rustfmt::skip]
        let mut board = Board::from_values(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9,
        ]);
        board.get_mut(Position::new(0, 1)).unwrap().removed = true;
        board.get_mut(Position::new(0, 3)).unwrap().removed = true;
        board.get_mut(Position::new(0, 5)).unwrap().removed = true;
        board.get_mut(Position::new(0, 7)).unwrap().removed = true;

        board.refill();

        assert!(board.get(Position::new(0, 1)).unwrap().removed);
        assert!(board.get(Position::new(0, 3)).unwrap().removed);
        assert!(board.get(Position::new(0, 5)).unwrap().removed);
        assert!(board.get(Position::new(0, 7)).unwrap().removed);

        assert_eq!(board.get(Position::new(1, 0)).unwrap().value, 1);
        assert_eq!(board.get(Position::new(1, 1)).unwrap().value, 3);
        assert_eq!(board.get(Position::new(1, 2)).unwrap().value, 5);
        assert_eq!(board.get(Position::new(1, 3)).unwrap().value, 7);
        assert_eq!(board.get(Position::new(1, 4)).unwrap().value, 9);
    }

    #[test]
    fn test_find_any_valid_pair_finds_pair() {
        let board = Board::from_values(&[5, 5, 0, 0, 0, 0, 0, 0, 0]);

        let pair = board.find_any_valid_pair();

        assert!(pair.is_some());
    }

    #[test]
    fn test_find_any_valid_pair_no_pair() {
        let board = Board::from_values(&[1, 2, 3, 0, 0, 0, 0, 0, 0]);

        let pair = board.find_any_valid_pair();

        assert!(pair.is_none());
    }

    #[test]
    fn test_active_cell_count() {
        let board = Board::from_values(&[1, 0, 2, 0, 3, 0, 0, 0, 0]);

        assert_eq!(board.active_cell_count(), 3);
    }

    #[test]
    fn test_fuzz_no_adjacent_horizontal_duplicates() {
        for seed in 0..1000u64 {
            let mut rng = SessionRng::new(seed);
            let board = Board::new(&mut rng);

            for row in 0..board.row_count() {
                for col in 1..FIELD_WIDTH {
                    let idx = row * FIELD_WIDTH + col;
                    if idx >= INITIAL_CELLS {
                        break;
                    }

                    let prev_idx = idx - 1;
                    let prev_cell = &board.cells[prev_idx];
                    let curr_cell = &board.cells[idx];

                    if prev_cell.is_active() && curr_cell.is_active() {
                        assert_ne!(
                            prev_cell.value, curr_cell.value,
                            "Seed {}: adjacent cells at row {}, cols {}-{} have same value {}",
                            seed, row, col - 1, col, prev_cell.value
                        );
                    }
                }
            }
        }
    }
}
