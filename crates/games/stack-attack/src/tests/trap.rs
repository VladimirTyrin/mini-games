//! A crate dropping into a worker's gap and the trapped ending (`docs/refactoring-plan.md`,
//! section 7).

use super::support::*;
use mg_core::{Game, Status};

use crate::{Cell, Command, Direction, EndReason, WorkerStatus};

/// Columns `0..width` filled up to `height(x)` resting boxes.
fn build_columns(game: &mut crate::StackAttack, heights: &[i32]) {
    for (x, &height) in (0..).zip(heights) {
        for y in 0..height {
            put_box(game, x, y);
        }
    }
}

#[test]
fn a_crate_falling_into_the_workers_gap_crushes_instead_of_completing_the_row() {
    // The screenshot case: every column but the worker's is filled; a crate falls into the gap.
    // A teammate stands free on top, otherwise the pit itself would end the game.
    let mut game = new_game(6, 10, 2);
    build_columns(&mut game, &[3, 3, 0, 3, 3, 3]);
    place_worker(&mut game, 0, 2, 0);
    place_worker(&mut game, 1, 4, 3);
    let crate_id = put_falling_box(&mut game, 2, 3);

    steps(&mut game, 2);

    assert_eq!(worker(&game, 0).status, WorkerStatus::Crushed);
    assert_eq!(view(&game).end_reason, None);
    assert_eq!(view(&game).lines_cleared, 0);
    assert_eq!(view(&game).score.get(), 0);
    assert!(box_cell(&game, crate_id).is_some());
}

#[test]
fn a_row_still_clears_once_the_last_crate_lands() {
    let mut game = new_game(6, 10, 1);
    fill_row(&mut game, 0, &[2]);
    place_worker(&mut game, 0, 0, 1);
    put_falling_box(&mut game, 2, 2);

    steps(&mut game, 2);
    assert_eq!(view(&game).lines_cleared, 0, "the crate is still falling");
    steps(&mut game, 1);
    assert_eq!(view(&game).lines_cleared, 1);
}

#[test]
fn a_pit_two_crates_deeper_than_both_sides_traps_the_worker() {
    let mut game = new_game(6, 10, 1);
    build_columns(&mut game, &[3, 2, 0, 2, 3, 3]);
    place_worker(&mut game, 0, 2, 0);

    steps(&mut game, 1);

    assert_eq!(view(&game).end_reason, Some(EndReason::WorkersTrapped));
    assert!(matches!(game.status(), Status::Finished(_)));
}

#[test]
fn a_one_crate_step_is_climbable_so_the_worker_is_not_trapped() {
    let mut game = new_game(6, 10, 1);
    build_columns(&mut game, &[3, 2, 0, 1, 3, 3]);
    place_worker(&mut game, 0, 2, 0);

    steps(&mut game, 1);
    assert_eq!(view(&game).end_reason, None);

    send(&mut game, 0, Command::Jump);
    send(&mut game, 0, Command::Move(Direction::Right));
    steps(&mut game, 3);
    assert_eq!(worker(&game, 0).feet, Cell::new(3, 1));
}

#[test]
fn a_crate_that_can_be_pushed_away_is_not_a_wall() {
    // Wall on the left; on the right a single crate at the feet with a free cell behind it.
    let mut game = new_game(6, 10, 1);
    build_columns(&mut game, &[3, 2, 0, 1, 0, 3]);
    put_box(&mut game, 3, 1);
    place_worker(&mut game, 0, 2, 0);
    // The right column is two high, but its top crate can be pushed into column 4 after a jump.
    steps(&mut game, 1);
    assert_eq!(view(&game).end_reason, None);
}

#[test]
fn a_crate_falling_past_the_pit_does_not_count_as_a_wall() {
    let mut game = new_game(6, 10, 1);
    build_columns(&mut game, &[3, 2, 0, 0, 3, 3]);
    place_worker(&mut game, 0, 2, 0);
    put_falling_box(&mut game, 3, 3);

    // It passes the worker's head height while falling; only once it rests is the pit closed
    // on that side, and a single crate is a climbable step.
    steps(&mut game, 5);
    assert_eq!(view(&game).end_reason, None);
}

#[test]
fn one_free_worker_keeps_a_trapped_teammate_in_the_game() {
    let mut game = new_game(8, 10, 2);
    build_columns(&mut game, &[3, 2, 0, 2, 3, 0, 0, 0]);
    place_worker(&mut game, 0, 2, 0);
    place_worker(&mut game, 1, 6, 0);

    steps(&mut game, 1);
    assert_eq!(view(&game).end_reason, None);
    assert_eq!(worker(&game, 0).status, WorkerStatus::Alive);
}
