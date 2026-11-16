//! Legacy `board.rs` tests (ported) plus board-level additions.

use super::support::{at, board, digit_at, pair, value};
use crate::Cell;
use crate::model::{Board, RowIndex};

#[test]
fn test_board_from_values_creates_correct_layout() {
    let digits: Vec<u8> = (0..42).map(|i| (i % 9) + 1).collect();
    let board = board(&digits);
    assert_eq!(board.rows(), 5);
    assert_eq!(digit_at(&board, 0, 0), 1);
    assert_eq!(digit_at(&board, 0, 8), 9);
}

#[test]
fn test_can_remove_pair_equal_values_horizontal() {
    let board = board(&[5, 0, 0, 5, 0, 0, 0, 0, 0]);
    assert!(board.can_remove_pair(pair(at(0, 0), at(0, 3))));
}

#[test]
fn test_can_remove_pair_sum_ten_horizontal() {
    let board = board(&[3, 0, 7, 0, 0, 0, 0, 0, 0]);
    assert!(board.can_remove_pair(pair(at(0, 0), at(0, 2))));
}

#[test]
fn test_can_remove_pair_blocked_by_other_cell() {
    let board = board(&[5, 1, 5, 0, 0, 0, 0, 0, 0]);
    assert!(!board.can_remove_pair(pair(at(0, 0), at(0, 2))));
}

#[test]
fn test_can_remove_pair_diagonal() {
    #[rustfmt::skip]
    let board = board(&[
        5, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 5, 0, 0, 0, 0, 0, 0,
    ]);
    assert!(board.can_remove_pair(pair(at(0, 0), at(2, 2))));
}

#[test]
fn test_can_remove_pair_diagonal_blocked() {
    #[rustfmt::skip]
    let board = board(&[
        5, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 5, 0, 0, 0, 0, 0, 0,
    ]);
    assert!(!board.can_remove_pair(pair(at(0, 0), at(2, 2))));
}

#[test]
fn test_can_remove_pair_vertical() {
    #[rustfmt::skip]
    let board = board(&[
        5, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0,
        5, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);
    assert!(board.can_remove_pair(pair(at(0, 0), at(2, 0))));
}

#[test]
fn test_can_remove_pair_sequential_path() {
    #[rustfmt::skip]
    let board = board(&[
        5, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 5, 0, 0, 0, 0, 0, 0,
    ]);
    assert!(board.can_remove_pair(pair(at(0, 0), at(1, 2))));
}

#[test]
fn test_can_remove_pair_sequential_path_blocked() {
    #[rustfmt::skip]
    let board = board(&[
        5, 0, 0, 0, 1, 0, 0, 0, 0,
        0, 0, 5, 0, 0, 0, 0, 0, 0,
    ]);
    assert!(!board.can_remove_pair(pair(at(0, 0), at(1, 2))));
}

#[test]
fn test_can_remove_pair_same_cell() {
    let board = board(&[5, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert!(!board.can_remove_pair(pair(at(0, 0), at(0, 0))));
}

#[test]
fn test_can_remove_pair_different_values_not_sum_ten() {
    let board = board(&[3, 5, 0, 0, 0, 0, 0, 0, 0]);
    assert!(!board.can_remove_pair(pair(at(0, 0), at(0, 1))));
}

#[test]
fn test_remove_empty_rows_deletes_empty_row() {
    #[rustfmt::skip]
    let mut board = board(&[
        1, 2, 3, 4, 5, 6, 7, 8, 9,
        0, 0, 0, 0, 0, 0, 0, 0, 0,
        1, 2, 3, 4, 5, 6, 7, 8, 9,
    ]);
    assert_eq!(board.delete_inactive_rows(), vec![RowIndex::new(1)]);
    assert_eq!(board.rows(), 2);
}

#[test]
fn test_refill_copies_active_cells() {
    #[rustfmt::skip]
    let mut board = board(&[
        1, 0, 2, 0, 0, 0, 0, 0, 0,
        3, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 4, 0, 0, 0, 0, 0, 0, 0,
    ]);
    let (_, added) = board.refill();
    assert_eq!(added, vec![value(1), value(2), value(3), value(4)]);
}

#[test]
fn test_refill_example_from_task() {
    #[rustfmt::skip]
    let mut board = board(&[
        1, 0, 2, 0, 0, 0, 0, 0, 0,
        3, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 4, 0, 0, 0, 0, 0, 0, 0,
    ]);
    let (start, _) = board.refill();
    assert_eq!(start, at(2, 2));
    assert_eq!(digit_at(&board, 2, 2), 1);
    assert_eq!(digit_at(&board, 2, 3), 2);
    assert_eq!(digit_at(&board, 2, 4), 3);
    assert_eq!(digit_at(&board, 2, 5), 4);
}

#[test]
fn test_refill_writes_after_removed_cells() {
    let mut board = board(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    board.remove_pair(pair(at(0, 1), at(0, 3)));
    board.remove_pair(pair(at(0, 5), at(0, 7)));

    board.refill();

    for column in [1, 3, 5, 7] {
        assert!(matches!(
            board.cells()[at(0, column).as_usize()],
            Cell::Removed(_)
        ));
    }
    assert_eq!(digit_at(&board, 1, 0), 1);
    assert_eq!(digit_at(&board, 1, 1), 3);
    assert_eq!(digit_at(&board, 1, 2), 5);
    assert_eq!(digit_at(&board, 1, 3), 7);
    assert_eq!(digit_at(&board, 1, 4), 9);
    assert_eq!(board.rows(), 2);
}

#[test]
fn test_find_any_valid_pair_finds_pair() {
    let board = board(&[5, 5, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(board.find_pair(), Some(pair(at(0, 0), at(0, 1))));
}

#[test]
fn test_find_any_valid_pair_no_pair() {
    let board = board(&[1, 2, 3, 0, 0, 0, 0, 0, 0]);
    assert_eq!(board.find_pair(), None);
}

#[test]
fn test_active_cell_count() {
    let board = board(&[1, 0, 2, 0, 3, 0, 0, 0, 0]);
    assert_eq!(board.active_count(), 3);
}

// Legacy `test_fuzz_no_adjacent_horizontal_duplicates` is in `game_rules`: dealing is a world
// event now.

#[test]
fn deleted_rows_are_reported_in_pre_deletion_indices() {
    #[rustfmt::skip]
    let mut board = board(&[
        1, 2, 3, 4, 5, 6, 7, 8, 9,
        0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0,
        1, 2, 3, 4, 5, 6, 7, 8, 9,
    ]);
    assert_eq!(
        board.delete_inactive_rows(),
        vec![RowIndex::new(1), RowIndex::new(2)]
    );
    assert_eq!(board.rows(), 2);
    assert_eq!(digit_at(&board, 1, 0), 1);
}

#[test]
fn removed_cells_are_no_longer_matchable_and_unblock_paths() {
    let mut board = board(&[5, 1, 9, 5, 0, 0, 0, 0, 0]);
    assert!(!board.can_remove_pair(pair(at(0, 0), at(0, 3))));
    board.remove_pair(pair(at(0, 1), at(0, 2)));
    assert!(!board.can_remove_pair(pair(at(0, 1), at(0, 2))));
    assert!(board.can_remove_pair(pair(at(0, 3), at(0, 0))));
}

#[test]
fn out_of_range_cells_are_rejected() {
    let board = board(&[5, 5, 0, 0, 0, 0, 0, 0, 0]);
    assert!(!board.can_remove_pair(pair(at(0, 0), at(100, 0))));
    assert!(!board.can_remove_pair(pair(at(0, 0), at(0, 2))));
}

#[test]
fn diagonals_do_not_wrap_around_rows() {
    // (1,8) and (3,0) are 10 cells apart like a down-right diagonal step, but they are not on a
    // line, and row 2 blocks the reading-order path.
    let mut cells = vec![Cell::Removed(value(9)); 36];
    cells[at(1, 8).as_usize()] = Cell::Active(value(5));
    cells[at(3, 0).as_usize()] = Cell::Active(value(5));
    for column in 0..9 {
        let digit = if column % 2 == 0 { 1 } else { 2 };
        cells[at(2, column).as_usize()] = Cell::Active(value(digit));
    }
    let board = Board::from_cells(cells);
    assert!(!board.can_remove_pair(pair(at(1, 8), at(3, 0))));
    assert_eq!(board.find_pair(), None);
}

#[test]
fn finder_sees_anti_diagonals_across_removed_cells() {
    let mut cells = vec![Cell::Removed(value(1)); 27];
    cells[at(0, 4).as_usize()] = Cell::Active(value(3));
    cells[at(2, 2).as_usize()] = Cell::Active(value(7));
    cells[at(1, 0).as_usize()] = Cell::Active(value(1));
    cells[at(2, 8).as_usize()] = Cell::Active(value(2));
    let board = Board::from_cells(cells);
    assert_eq!(board.find_pair(), Some(pair(at(0, 4), at(2, 2))));
}
