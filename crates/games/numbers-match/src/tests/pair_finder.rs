//! N3: the linear pair finder must return exactly what legacy's brute-force scan returned.

use mg_core::CoreRng;

use super::support::value;
use crate::board::Board;
use crate::{COLUMNS, Cell};

/// `active_per_16`/16 of the cells active, the rest removed; up to 8 empty tail cells.
fn random_board(rng: &mut CoreRng, max_rows: u32, active_per_16: u32) -> Board {
    let rows = rng.range_inclusive(1, max_rows) as usize;
    let largest_digit = rng.range_inclusive(2, 9);
    let length = rows * COLUMNS - rng.below(COLUMNS as u32) as usize;
    let cells = (0..length)
        .map(|_| {
            let digit = value(rng.range_inclusive(1, largest_digit) as u8);
            if rng.chance(active_per_16, 16) {
                Cell::Active(digit)
            } else {
                Cell::Removed(digit)
            }
        })
        .collect();
    Board::from_cells(cells)
}

#[test]
fn fast_finder_matches_brute_force_on_random_boards() {
    let mut rng = CoreRng::new(0x5EED);
    let mut boards_with_pair = 0;
    for _ in 0..20_000 {
        let active_per_16 = rng.below(17);
        let board = random_board(&mut rng, 14, active_per_16);
        let fast = board.find_pair();
        assert_eq!(fast, board.find_pair_brute_force(), "{:?}", board.cells());
        if let Some(pair) = fast {
            assert!(board.can_remove_pair(pair));
            boards_with_pair += 1;
        }
    }
    // Both outcomes must be well represented for the comparison to mean anything.
    assert!(
        (2_000..18_000).contains(&boards_with_pair),
        "{boards_with_pair}"
    );
}

#[test]
fn fast_finder_matches_brute_force_on_sparse_boards() {
    // Few active cells: long-range lines and reading-order paths across many rows.
    let mut rng = CoreRng::new(77);
    for _ in 0..20_000 {
        let board = random_board(&mut rng, 10, 2);
        assert_eq!(board.find_pair(), board.find_pair_brute_force());
    }
}
