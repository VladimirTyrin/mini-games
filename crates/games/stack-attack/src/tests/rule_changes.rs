//! One test per rule change against the legacy server (`docs/refactoring-plan.md`, section 7).

use mg_core::{CommandError, Game, PlayerSpec, SetupError, Status};

use super::support::*;
use crate::{
    Cell, Column, Command, CraneState, Direction, EndReason, GameEvent, MAX_QUEUED_COMMANDS, Row,
    Settings, StackAttack, TickCount, WorkerStatus,
};

const RIGHT: Command = Command::Move(Direction::Right);

#[test]
fn a3_one_action_per_tick_and_a_bounded_queue() {
    let mut game = new_game(8, 8, 1);
    place_worker(&mut game, 0, 1, 0);
    for _ in 0..MAX_QUEUED_COMMANDS {
        send(&mut game, 0, RIGHT);
    }
    let before = game.state_hash();
    assert_eq!(game.apply_command(slot(0), &RIGHT), Err(CommandError::Busy));
    assert_eq!(game.state_hash(), before);
    assert_eq!(worker(&game, 0).feet.x, 1, "commands only run in step");

    for expected_x in 2..=4 {
        steps(&mut game, 1);
        assert_eq!(worker(&game, 0).feet.x, expected_x);
    }
    assert_eq!(worker(&game, 0).queued_commands, 0);
}

#[test]
fn a1_pushed_box_without_support_falls() {
    let mut game = new_game(8, 8, 1);
    put_box(&mut game, 3, 0);
    put_box(&mut game, 4, 0);
    let pushed = put_box(&mut game, 4, 1);
    place_worker(&mut game, 0, 3, 1);
    send(&mut game, 0, RIGHT);
    steps(&mut game, 2);
    assert_eq!(box_cell(&game, pushed), Some(Cell::new(5, 0)));
    assert!(view(&game).events.contains(&GameEvent::BoxLanded {
        box_id: pushed,
        cell: Cell::new(5, 0),
    }));
    assert_eq!(worker(&game, 0).feet, Cell::new(4, 1));
}

/// Only resting boxes complete a row, so a clear can no longer leave a box hanging over a hole
/// in play; the field-level test `line_clear_makes_boxes_above_a_hole_fall` keeps that guard.
/// Here: a crate still falling when the row below it clears is carried down and completes the
/// next row once it rests.
#[test]
fn a9_boxes_shifted_by_a_line_clear_keep_falling() {
    let mut game = new_game(8, 8, 1);
    fill_row(&mut game, 0, &[2]);
    fill_row(&mut game, 1, &[2]);
    put_falling_box(&mut game, 2, 1);
    let upper = put_falling_box(&mut game, 2, 3);
    place_worker(&mut game, 0, 6, 2);

    steps(&mut game, 2);
    assert_eq!(view(&game).lines_cleared, 1);
    assert_eq!(box_cell(&game, upper), Some(Cell::new(2, 0)));

    steps(&mut game, 1);
    assert_eq!(
        view(&game).lines_cleared,
        2,
        "it came to rest in row 0 and completed it"
    );
}

#[test]
fn a2_two_cranes_never_drop_into_the_same_cell() {
    let mut game = new_game(8, 8, 1);
    place_worker(&mut game, 0, 6, 0);
    let first = add_crane(&mut game, 2, 3);
    let second = add_crane(&mut game, 2, 3);

    // Both arrive over the column and hold their crates for a tick.
    steps(&mut game, 1);
    assert!(view(&game).boxes.is_empty());
    steps(&mut game, 1);
    let cranes = view(&game).cranes;
    assert_eq!(cranes[0].id, first);
    assert_eq!(cranes[0].state, CraneState::Dropping);
    assert_eq!(cranes[1].id, second);
    assert_eq!(
        (cranes[1].x, cranes[1].state),
        (Column::new(3), CraneState::Moving)
    );
    assert_eq!(view(&game).boxes.len(), 1);

    steps(&mut game, 1);
    assert_eq!(view(&game).cranes[1].state, CraneState::Dropping);
    assert_eq!(box_cells(&game), vec![Cell::new(3, 5), Cell::new(3, 6)]);
    game.field.check_consistency().unwrap();
}

#[test]
fn a2_crane_waits_while_the_top_cell_is_taken() {
    let mut game = new_game(8, 8, 1);
    place_worker(&mut game, 0, 6, 0);
    put_falling_box(&mut game, 3, 7);
    add_crane(&mut game, 2, 3);
    steps(&mut game, 1);
    assert_eq!(view(&game).cranes[0].state, CraneState::Moving);
    assert_eq!(view(&game).boxes.len(), 1);
    steps(&mut game, 1);
    assert_eq!(view(&game).cranes[0].state, CraneState::Dropping);
    assert_eq!(view(&game).boxes.len(), 2);
}

#[test]
fn a6_line_clear_happens_before_the_ceiling_check() {
    let mut game = new_game(8, 6, 1);
    fill_row(&mut game, 0, &[0]);
    for y in 1..5 {
        put_box(&mut game, 1, y);
    }
    put_falling_box(&mut game, 1, 5);
    // Lands this tick, together with the crate reaching the top row.
    put_falling_box(&mut game, 0, 0);
    place_worker(&mut game, 0, 5, 1);

    steps(&mut game, 1);
    assert_eq!(view(&game).lines_cleared, 1);
    assert_eq!(view(&game).end_reason, None);
    assert_eq!(game.status(), Status::InProgress);
}

#[test]
fn a7_reported_points_equal_the_score_increase() {
    let mut game = new_game(8, 8, 1);
    fill_row(&mut game, 0, &[2]);
    fill_row(&mut game, 1, &[2]);
    put_falling_box(&mut game, 2, 0);
    put_falling_box(&mut game, 2, 1);
    place_worker(&mut game, 0, 6, 2);

    steps(&mut game, 1);
    let view = view(&game);
    assert_eq!(view.score.get(), 250);
    let reported: u32 = view
        .events
        .iter()
        .map(|event| match event {
            GameEvent::LinesCleared { points, .. } => *points,
            _ => 0,
        })
        .sum();
    assert_eq!(reported, 250);
    assert!(view.events.contains(&GameEvent::LinesCleared {
        rows: vec![Row::new(0), Row::new(1)],
        points: 250,
    }));
}

#[test]
fn a10_survival_time_is_counted_in_ticks() {
    let mut game = new_game(8, 8, 1);
    steps(&mut game, 7);
    game.drop_player(slot(0));
    let view = view(&game);
    assert_eq!(view.ticks_survived, TickCount::new(7));
    assert_eq!(view.tick_ms, 200);
    assert_eq!(view.end_reason, Some(EndReason::AllWorkersLeft));
}

#[test]
fn a11_leaving_removes_only_that_worker() {
    let mut game = new_game(8, 8, 2);
    game.drop_player(slot(0));
    assert_eq!(game.status(), Status::InProgress);
    assert_eq!(worker(&game, 0).status, WorkerStatus::Left);
    assert_eq!(worker(&game, 1).status, WorkerStatus::Alive);
    assert_eq!(
        game.apply_command(slot(0), &RIGHT),
        Err(CommandError::NotPlaying)
    );
    steps(&mut game, 1);
    assert!(
        view(&game)
            .events
            .contains(&GameEvent::WorkerLeft { slot: slot(0) })
    );
    send(&mut game, 1, RIGHT);

    game.drop_player(slot(1));
    let Status::Finished(outcome) = game.status() else {
        panic!("no worker left, the game must end");
    };
    assert_eq!(outcome.scores.len(), 2);
    assert_eq!(view(&game).end_reason, Some(EndReason::AllWorkersLeft));
    assert!(
        view(&game)
            .events
            .contains(&GameEvent::WorkerLeft { slot: slot(1) })
    );
}

#[test]
fn boxes_cannot_be_pushed_into_another_worker() {
    let mut game = new_game(8, 8, 2);
    place_worker(&mut game, 0, 2, 0);
    let between = put_box(&mut game, 3, 0);
    place_worker(&mut game, 1, 4, 0);
    send(&mut game, 0, RIGHT);
    steps(&mut game, 1);
    assert_eq!(worker(&game, 0).feet, Cell::new(2, 0));
    assert_eq!(box_cell(&game, between), Some(Cell::new(3, 0)));
    assert_eq!(worker(&game, 1).status, WorkerStatus::Alive);
}

#[test]
fn workers_may_still_overlap() {
    let mut game = new_game(8, 8, 2);
    place_worker(&mut game, 0, 2, 0);
    place_worker(&mut game, 1, 3, 0);
    send(&mut game, 0, RIGHT);
    steps(&mut game, 1);
    assert_eq!(worker(&game, 0).feet, worker(&game, 1).feet);
}

#[test]
fn bot_seats_are_rejected() {
    let result = StackAttack::new(&Settings::default(), &[PlayerSpec::HUMAN, PlayerSpec::BOT]);
    assert!(matches!(result, Err(SetupError::InvalidPlayers(_))));
}
