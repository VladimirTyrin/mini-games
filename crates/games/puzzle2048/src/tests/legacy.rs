//! Ports of the 17 legacy `game_state.rs` tests.

use mg_core::{CommandError, CoreRng, Game, OutcomeKind, Status};

use super::support::{board, play, settings, spawn, started, tile_count, try_move, values, view};
use crate::{Direction, Phase, Refusal, Settings, SpawnValue};

#[rustfmt::skip]
const EMPTY_4X4: [u32; 16] = [
    0, 0, 0, 0,
    0, 0, 0, 0,
    0, 0, 0, 0,
    0, 0, 0, 0,
];

fn with(cells: &[(usize, u32)]) -> Vec<u32> {
    let mut values = EMPTY_4X4.to_vec();
    for &(index, value) in cells {
        values[index] = value;
    }
    values
}

#[test]
fn new_game_has_two_tiles() {
    let game = started(&Settings::default(), &mut CoreRng::new(42));
    assert_eq!(tile_count(&game), 2);
}

#[test]
fn move_left_merges_equal() {
    let mut game = board(4, 4, &with(&[(0, 2), (1, 2)]));
    try_move(&mut game, Direction::Left).unwrap();
    assert_eq!(values(&game), with(&[(0, 4)]));
}

#[test]
fn move_right_merges_equal() {
    let mut game = board(4, 4, &with(&[(2, 2), (3, 2)]));
    try_move(&mut game, Direction::Right).unwrap();
    assert_eq!(values(&game), with(&[(3, 4)]));
}

#[test]
fn move_up_merges() {
    let mut game = board(4, 4, &with(&[(0, 2), (4, 2)]));
    try_move(&mut game, Direction::Up).unwrap();
    assert_eq!(values(&game), with(&[(0, 4)]));
}

#[test]
fn move_down_merges() {
    let mut game = board(4, 4, &with(&[(8, 2), (12, 2)]));
    try_move(&mut game, Direction::Down).unwrap();
    assert_eq!(values(&game), with(&[(12, 4)]));
}

#[test]
fn move_spawns_a_new_tile() {
    let mut game = board(4, 4, &with(&[(0, 2), (1, 2)]));
    play(&mut game, Direction::Left, &mut CoreRng::new(42)).unwrap();
    // Two tiles merged into one, then one spawned.
    assert_eq!(tile_count(&game), 2);
}

#[test]
fn move_that_changes_nothing_is_rejected() {
    let mut game = board(4, 4, &with(&[(0, 2)]));
    let before = game.state_hash();
    assert!(matches!(
        try_move(&mut game, Direction::Left),
        Err(CommandError::Illegal(Refusal::MoveChangesNothing))
    ));
    assert_eq!(game.state_hash(), before);
    assert_eq!(game.moves(), 0);
}

#[test]
fn won_when_target_reached() {
    let mut game = board(4, 4, &with(&[(0, 1024), (1, 1024)]));
    try_move(&mut game, Direction::Left).unwrap();
    spawn(&mut game, 3, 3, SpawnValue::Two).unwrap();
    assert_eq!(game.phase(), Phase::Won);
    assert!(matches!(game.status(), Status::Finished(o) if o.kind == OutcomeKind::Victory));
}

/// Legacy version asserted nothing. A move that leaves a single gap whose neighbours can never
/// match a 2 or a 4 must lose whatever spawns there.
#[test]
fn lost_when_no_moves() {
    for value in [SpawnValue::Two, SpawnValue::Four] {
        #[rustfmt::skip]
        let mut game = board(3, 2, &[
            8, 16, 8,
            0, 32, 64,
        ]);
        try_move(&mut game, Direction::Left).unwrap();
        assert_eq!(
            game.status(),
            Status::InProgress,
            "decided only after the spawn"
        );
        spawn(&mut game, 1, 2, value).unwrap();
        assert_eq!(game.phase(), Phase::Lost);
        let Status::Finished(outcome) = game.status() else {
            panic!("game must be over");
        };
        assert_eq!(outcome.kind, OutcomeKind::Defeat);
    }
}

/// Legacy version only asserted under a condition that never held. Both possible spawns are
/// now checked explicitly.
#[test]
fn lost_when_board_full_and_no_merges() {
    #[rustfmt::skip]
    let start = [
        2, 4,
        8, 8,
    ];
    let mut game = board(2, 2, &start);
    try_move(&mut game, Direction::Left).unwrap();
    spawn(&mut game, 1, 1, SpawnValue::Two).unwrap();
    assert_eq!(values(&game), [2, 4, 16, 2]);
    assert_eq!(game.phase(), Phase::Lost);

    let mut game = board(2, 2, &start);
    try_move(&mut game, Direction::Left).unwrap();
    spawn(&mut game, 1, 1, SpawnValue::Four).unwrap();
    assert_eq!(
        game.phase(),
        Phase::Playing,
        "the spawned 4 can merge upwards"
    );
}

#[test]
fn score_incremented_on_merge() {
    let mut game = board(4, 4, &with(&[(0, 2), (1, 2)]));
    try_move(&mut game, Direction::Left).unwrap();
    assert_eq!(game.score().get(), 4);
}

#[test]
fn custom_board_size() {
    let game = started(&settings(5, 6, 2048), &mut CoreRng::new(42));
    let view = view(&game);
    assert_eq!((view.width, view.height), (5, 6));
    assert_eq!(view.cell_values().len(), 30);
    assert_eq!(view.tiles.len(), 2);
}

/// Slides one 4-cell row to the left; returns the row and the score gained.
fn slide_row(row: [u32; 4]) -> (Vec<u32>, u64) {
    let mut cells = row.to_vec();
    cells.extend([0; 4]);
    let mut game = board(4, 2, &cells);
    match try_move(&mut game, Direction::Left) {
        Ok(()) => {}
        Err(CommandError::Illegal(_)) => {}
        Err(other) => panic!("unexpected rejection {other:?}"),
    }
    (values(&game)[..4].to_vec(), game.score().get())
}

#[test]
fn slide_line_basic() {
    assert_eq!(slide_row([2, 2, 0, 0]), (vec![4, 0, 0, 0], 4));
}

#[test]
fn slide_line_no_merge() {
    assert_eq!(slide_row([2, 4, 8, 16]), (vec![2, 4, 8, 16], 0));
}

#[test]
fn slide_line_double_merge() {
    assert_eq!(slide_row([2, 2, 4, 4]), (vec![4, 8, 0, 0], 12));
}

#[test]
fn slide_line_triple_same() {
    assert_eq!(slide_row([2, 2, 2, 0]), (vec![4, 2, 0, 0], 4));
}

#[test]
fn moves_made_incremented() {
    let mut game = board(4, 4, &with(&[(0, 2), (1, 2)]));
    assert_eq!(game.moves(), 0);
    try_move(&mut game, Direction::Left).unwrap();
    assert_eq!(game.moves(), 1);
}
