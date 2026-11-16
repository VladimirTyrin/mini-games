//! Behaviour carried over unchanged from the legacy server.

use mg_core::{CommandError, CoreRng, Game, OutcomeKind, Status};

use super::support::*;
use crate::{
    Cell, Column, Command, CraneState, Direction, EndReason, GameEvent, Level, Side, TickCount,
    WorkerState, WorkerStatus, WorldEvent,
};

const LEFT: Command = Command::Move(Direction::Left);
const RIGHT: Command = Command::Move(Direction::Right);

#[test]
fn workers_spawn_on_the_floor_spread_evenly() {
    let expected = [vec![8], vec![5, 10], vec![4, 8, 11], vec![3, 6, 9, 12]];
    for (seats, columns) in (1..=4).zip(expected) {
        let game = new_game(15, 10, seats);
        let workers = view(&game).workers;
        let got: Vec<i32> = workers.iter().map(|w| w.feet.x).collect();
        assert_eq!(got, columns, "{seats} seats");
        assert!(workers.iter().all(|w| w.feet.y == 0
            && w.state == WorkerState::Grounded
            && w.status == WorkerStatus::Alive));
    }
}

#[test]
fn worker_walks_one_cell_per_move_and_stops_at_walls() {
    let mut game = new_game(8, 8, 1);
    place_worker(&mut game, 0, 1, 0);
    for (command, expected_x) in [(LEFT, 0), (LEFT, 0), (RIGHT, 1), (RIGHT, 2)] {
        send(&mut game, 0, command);
        steps(&mut game, 1);
        assert_eq!(worker(&game, 0).feet, Cell::new(expected_x, 0));
    }
    place_worker(&mut game, 0, 7, 0);
    send(&mut game, 0, RIGHT);
    steps(&mut game, 1);
    assert_eq!(worker(&game, 0).feet, Cell::new(7, 0));
}

#[test]
fn box_at_head_level_blocks_walking() {
    let mut game = new_game(8, 8, 1);
    place_worker(&mut game, 0, 1, 0);
    put_box(&mut game, 2, 0);
    put_box(&mut game, 2, 1);
    send(&mut game, 0, RIGHT);
    steps(&mut game, 1);
    assert_eq!(worker(&game, 0).feet, Cell::new(1, 0));
    assert_eq!(view(&game).boxes_pushed, 0);
}

#[test]
fn walking_into_a_box_pushes_it() {
    let mut game = new_game(8, 8, 1);
    place_worker(&mut game, 0, 1, 0);
    let pushed = put_box(&mut game, 2, 0);
    send(&mut game, 0, RIGHT);
    steps(&mut game, 1);
    let view = view(&game);
    assert_eq!(view.workers[0].feet, Cell::new(2, 0));
    assert_eq!(box_cell(&game, pushed), Some(Cell::new(3, 0)));
    assert_eq!(view.boxes_pushed, 1);
    assert!(view.events.contains(&GameEvent::BoxPushed {
        box_id: pushed,
        from_x: Column::new(2),
        to_x: Column::new(3),
        by: slot(0),
    }));
}

#[test]
fn push_is_blocked_by_another_box_or_the_wall() {
    let mut game = new_game(8, 8, 1);
    place_worker(&mut game, 0, 4, 0);
    put_box(&mut game, 5, 0);
    put_box(&mut game, 6, 0);
    send(&mut game, 0, RIGHT);
    steps(&mut game, 1);
    assert_eq!(worker(&game, 0).feet, Cell::new(4, 0));

    let mut game = new_game(8, 8, 1);
    place_worker(&mut game, 0, 6, 0);
    put_box(&mut game, 7, 0);
    send(&mut game, 0, RIGHT);
    steps(&mut game, 1);
    assert_eq!(worker(&game, 0).feet, Cell::new(6, 0));
    assert_eq!(box_cells(&game), vec![Cell::new(7, 0)]);
}

#[test]
fn jump_lifts_for_one_tick_then_falls_back() {
    let mut game = new_game(8, 8, 1);
    send(&mut game, 0, Command::Jump);
    steps(&mut game, 1);
    assert_eq!(worker(&game, 0).feet.y, 1);
    assert!(
        view(&game)
            .events
            .contains(&GameEvent::WorkerJumped { slot: slot(0) })
    );
    steps(&mut game, 1);
    assert_eq!(worker(&game, 0).feet.y, 0);
    assert_eq!(worker(&game, 0).state, WorkerState::Falling);
    steps(&mut game, 1);
    assert_eq!(worker(&game, 0).state, WorkerState::Grounded);
    assert!(view(&game).events.contains(&GameEvent::WorkerLanded {
        slot: slot(0),
        feet: Cell::new(4, 0),
    }));
}

#[test]
fn jump_needs_ground_and_headroom() {
    let mut game = new_game(8, 8, 1);
    send(&mut game, 0, Command::Jump);
    send(&mut game, 0, Command::Jump);
    steps(&mut game, 2);
    assert_eq!(
        worker(&game, 0).feet.y,
        0,
        "second jump happened in the air"
    );

    let mut game = new_game(8, 6, 1);
    for y in 0..4 {
        put_box(&mut game, 4, y);
    }
    place_worker(&mut game, 0, 4, 4);
    send(&mut game, 0, Command::Jump);
    steps(&mut game, 1);
    assert_eq!(worker(&game, 0).feet.y, 4, "jumped through the ceiling");
}

#[test]
fn no_jump_off_a_row_that_was_just_cleared() {
    let mut game = new_game(8, 8, 1);
    fill_row(&mut game, 0, &[2]);
    put_falling_box(&mut game, 2, 1);
    place_worker(&mut game, 0, 4, 1);
    // The crate settles into the gap and the row clears, leaving the worker standing on nothing.
    steps(&mut game, 2);
    assert_eq!(view(&game).lines_cleared, 1);

    send(&mut game, 0, Command::Jump);
    steps(&mut game, 1);

    assert_eq!(worker(&game, 0).feet.y, 0, "jumped off thin air");
}

#[test]
fn jump_then_move_climbs_onto_a_box() {
    let mut game = new_game(8, 8, 1);
    place_worker(&mut game, 0, 3, 0);
    put_box(&mut game, 4, 0);
    send(&mut game, 0, Command::Jump);
    send(&mut game, 0, RIGHT);
    steps(&mut game, 3);
    let worker = worker(&game, 0);
    assert_eq!(worker.feet, Cell::new(4, 1));
    assert_eq!(worker.state, WorkerState::Grounded);
}

#[test]
fn boxes_fall_one_cell_per_tick_and_land() {
    let mut game = new_game(8, 8, 1);
    let falling = put_falling_box(&mut game, 1, 3);
    for expected_y in [2, 1, 0] {
        steps(&mut game, 1);
        assert_eq!(box_cell(&game, falling), Some(Cell::new(1, expected_y)));
        assert!(view(&game).boxes[0].falling);
    }
    steps(&mut game, 1);
    assert!(!view(&game).boxes[0].falling);
    assert!(view(&game).events.contains(&GameEvent::BoxLanded {
        box_id: falling,
        cell: Cell::new(1, 0),
    }));
}

#[test]
fn falling_box_crushes_the_worker_below() {
    let mut game = new_game(8, 8, 2);
    let first = worker(&game, 0).feet;
    let crusher = put_falling_box(&mut game, first.x, 3);
    steps(&mut game, 1);
    assert_eq!(worker(&game, 0).status, WorkerStatus::Alive);
    steps(&mut game, 1);
    assert_eq!(worker(&game, 0).status, WorkerStatus::Crushed);
    assert!(view(&game).events.contains(&GameEvent::WorkerCrushed {
        slot: slot(0),
        box_id: crusher,
    }));
    assert_eq!(game.status(), Status::InProgress);
    assert_eq!(
        game.apply_command(slot(0), &RIGHT),
        Err(CommandError::NotPlaying)
    );

    let second = worker(&game, 1).feet;
    put_falling_box(&mut game, second.x, 2);
    steps(&mut game, 1);
    assert_eq!(view(&game).end_reason, Some(EndReason::WorkersCrushed));
}

#[test]
fn full_row_scores_and_shifts_the_rows_above_down() {
    let mut game = new_game(8, 8, 1);
    fill_row(&mut game, 0, &[]);
    let above = put_box(&mut game, 2, 1);
    place_worker(&mut game, 0, 6, 1);
    steps(&mut game, 1);
    let view = view(&game);
    assert_eq!(view.score.get(), 100);
    assert_eq!(view.lines_cleared, 1);
    assert_eq!(box_cells(&game), vec![Cell::new(2, 0)]);
    assert_eq!(box_cell(&game, above), Some(Cell::new(2, 0)));
    steps(&mut game, 1);
    assert_eq!(worker(&game, 0).feet, Cell::new(6, 0));
}

#[test]
fn crane_enters_drops_its_box_and_leaves() {
    let mut game = new_game(8, 8, 1);
    steps(&mut game, 15);
    let spawn = WorldEvent::CraneSpawned {
        target_x: Column::new(1),
        from: Side::Left,
        pattern: pattern(),
    };
    game.apply_world(&spawn).unwrap();
    let crane = |game: &crate::StackAttack| view(game).cranes.first().copied();
    assert_eq!(
        crane(&game).map(|c| (c.x, c.state)),
        Some((Column::new(-1), CraneState::Moving))
    );

    steps(&mut game, 1);
    assert_eq!(crane(&game).map(|c| c.x), Some(Column::new(0)));
    steps(&mut game, 1);
    assert_eq!(
        crane(&game).map(|c| (c.x, c.state)),
        Some((Column::new(1), CraneState::Dropping))
    );
    assert_eq!(box_cells(&game), vec![Cell::new(1, 6)]);
    steps(&mut game, 1);
    assert_eq!(
        crane(&game).map(|c| (c.x, c.state)),
        Some((Column::new(1), CraneState::Leaving))
    );
    steps(&mut game, 1);
    assert_eq!(crane(&game).map(|c| c.x), Some(Column::new(0)));
    steps(&mut game, 1);
    assert_eq!(crane(&game), None, "crane left through the left edge");
}

#[test]
fn cranes_spawn_after_the_interval_and_up_to_the_cap() {
    let mut game = new_game(32, 8, 1);
    let mut rng = CoreRng::new(7);
    steps(&mut game, 14);
    assert!(game.roll_world(&mut rng).is_empty());
    steps(&mut game, 1);
    assert_eq!(game.roll_world(&mut rng).len(), 1);
    let far_target = WorldEvent::CraneSpawned {
        target_x: Column::new(31),
        from: Side::Left,
        pattern: pattern(),
    };
    game.apply_world(&far_target).unwrap();
    steps(&mut game, 16);
    assert_eq!(view(&game).cranes.len(), 1);
    assert!(
        game.roll_world(&mut rng).is_empty(),
        "level 1 allows one crane at a time"
    );
}

#[test]
fn difficulty_grows_every_150_ticks() {
    let at = |ticks: u64| Level::after_ticks(TickCount::new(ticks));
    assert_eq!(at(0).get(), 1);
    assert_eq!(at(149).get(), 1);
    assert_eq!(at(150).get(), 2);
    let intervals: Vec<u32> = (0..8).map(|l| at(l * 150).crane_spawn_interval()).collect();
    assert_eq!(intervals, vec![15, 13, 11, 9, 7, 5, 4, 4]);
    let caps: Vec<u32> = (0..12).map(|l| at(l * 150).max_cranes()).collect();
    assert_eq!(caps, vec![1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 5, 5]);

    let mut game = new_game(8, 8, 1);
    steps(&mut game, 150);
    assert_eq!(view(&game).level.get(), 2);
}

#[test]
fn resting_box_in_the_top_row_ends_the_game() {
    let mut game = new_game(8, 6, 2);
    for y in 0..6 {
        put_box(&mut game, 0, y);
    }
    steps(&mut game, 1);
    assert_eq!(view(&game).end_reason, Some(EndReason::CeilingReached));
    let Status::Finished(outcome) = game.status() else {
        panic!("game should be over");
    };
    assert_eq!(outcome.kind, OutcomeKind::Defeat);
    assert!(outcome.winners.is_empty());
    assert_eq!(outcome.scores, vec![0, 0]);
    assert_eq!(
        game.apply_command(slot(0), &LEFT),
        Err(CommandError::GameOver)
    );
}

#[test]
fn invalid_crane_spawns_are_rejected_without_changes() {
    let mut game = new_game(8, 8, 1);
    let spawn = |target_x| WorldEvent::CraneSpawned {
        target_x: Column::new(target_x),
        from: Side::Right,
        pattern: pattern(),
    };
    let before = game.state_hash();
    assert!(game.apply_world(&spawn(3)).is_err(), "not due yet");
    steps(&mut game, 15);
    let before_due = game.state_hash();
    assert_ne!(before, before_due);
    assert!(game.apply_world(&spawn(8)).is_err());
    assert!(game.apply_world(&spawn(-1)).is_err());
    assert_eq!(game.state_hash(), before_due);
    assert_eq!(game.apply_world(&spawn(7)), Ok(()));
}

#[test]
fn pattern_ids_are_validated() {
    assert!(crate::PatternId::new(7).is_some());
    assert!(crate::PatternId::new(8).is_none());
}
