use mg_core::{CoreRng, Game, PlayerSpec};

use crate::board::Board;
use crate::{COLUMNS, Cell, CellIndex, CellPair, CellValue, HintMode, NumbersMatch, Settings};

pub(super) fn value(digit: u8) -> CellValue {
    CellValue::new(digit).unwrap()
}

/// Board from digits like legacy `Board::from_values`: `0` is an empty cell.
pub(super) fn board(digits: &[u8]) -> Board {
    Board::from_cells(
        digits
            .iter()
            .map(|&digit| match digit {
                0 => Cell::Empty,
                digit => Cell::Active(value(digit)),
            })
            .collect(),
    )
}

pub(super) fn at(row: usize, column: usize) -> CellIndex {
    CellIndex::from_usize(row * COLUMNS + column)
}

pub(super) fn pair(first: CellIndex, second: CellIndex) -> CellPair {
    CellPair::new(first, second)
}

pub(super) fn digit_at(board: &Board, row: usize, column: usize) -> u8 {
    match board.cells()[at(row, column).as_usize()] {
        Cell::Empty => 0,
        Cell::Active(value) | Cell::Removed(value) => value.get(),
    }
}

pub(super) fn settings(hint_mode: HintMode) -> Settings {
    Settings {
        hint_mode,
        ..Settings::default()
    }
}

pub(super) fn dealt_game_with_seed(settings: Settings, seed: u64) -> NumbersMatch {
    let mut game = NumbersMatch::new(&settings, &[PlayerSpec::HUMAN]).unwrap();
    let mut rng = CoreRng::new(seed);
    for event in game.roll_world(&mut rng) {
        game.apply_world(&event).unwrap();
    }
    game
}

/// Legacy tests used seed 12345.
pub(super) fn dealt_game(hint_mode: HintMode) -> NumbersMatch {
    dealt_game_with_seed(settings(hint_mode), 12345)
}
