//! Head hits: a jumping worker breaks falling crates (`docs/refactoring-plan.md`, section 7).

use super::support::*;
use crate::{
    Cell, Command, Direction, EndReason, GameEvent, StackAttack, WorkerState, WorkerStatus,
};

fn broken_events(game: &StackAttack) -> Vec<GameEvent> {
    view(game)
        .events
        .into_iter()
        .filter(|event| matches!(event, GameEvent::BoxBroken { .. }))
        .collect()
}

#[test]
fn jumping_into_a_falling_crate_breaks_it_for_no_points() {
    let mut game = new_game(8, 8, 1);
    place_worker(&mut game, 0, 3, 0);
    let crate_above = put_falling_box(&mut game, 3, 2);

    send(&mut game, 0, Command::Jump);
    steps(&mut game, 1);

    assert_eq!(box_cell(&game, crate_above), None);
    assert_eq!(
        broken_events(&game),
        vec![GameEvent::BoxBroken {
            box_id: crate_above,
            cell: Cell::new(3, 2),
            by: slot(0),
        }]
    );
    let worker = worker(&game, 0);
    assert_eq!(worker.status, WorkerStatus::Alive);
    assert_eq!(worker.feet, Cell::new(3, 1), "the jump went ahead");
    assert_eq!(view(&game).score.get(), 0);
}

#[test]
fn a_crate_that_would_fall_onto_the_rising_head_breaks() {
    let mut game = new_game(8, 8, 1);
    place_worker(&mut game, 0, 3, 0);
    let crate_two_above = put_falling_box(&mut game, 3, 3);

    send(&mut game, 0, Command::Jump);
    steps(&mut game, 1);

    assert_eq!(box_cell(&game, crate_two_above), None);
    assert_eq!(
        broken_events(&game),
        vec![GameEvent::BoxBroken {
            box_id: crate_two_above,
            cell: Cell::new(3, 3),
            by: slot(0),
        }]
    );
    assert_eq!(worker(&game, 0).status, WorkerStatus::Alive);
}

#[test]
fn a_jump_breaks_at_most_the_two_crates_it_meets() {
    let mut game = new_game(8, 8, 1);
    place_worker(&mut game, 0, 3, 0);
    let lower = put_falling_box(&mut game, 3, 2);
    let upper = put_falling_box(&mut game, 3, 3);
    let third = put_falling_box(&mut game, 3, 4);

    send(&mut game, 0, Command::Jump);
    steps(&mut game, 1);

    let broken: Vec<_> = broken_events(&game)
        .into_iter()
        .filter_map(|event| match event {
            GameEvent::BoxBroken { box_id, .. } => Some(box_id),
            _ => None,
        })
        .collect();
    assert_eq!(broken, vec![lower, upper]);
    assert_eq!(box_cell(&game, third), Some(Cell::new(3, 3)));
    assert_eq!(worker(&game, 0).status, WorkerStatus::Alive);
}

#[test]
fn a_worker_who_does_not_jump_is_still_crushed() {
    let mut game = new_game(8, 8, 1);
    place_worker(&mut game, 0, 3, 0);
    let crusher = put_falling_box(&mut game, 3, 2);

    steps(&mut game, 1);

    assert!(broken_events(&game).is_empty());
    assert_eq!(worker(&game, 0).status, WorkerStatus::Crushed);
    assert!(view(&game).events.contains(&GameEvent::WorkerCrushed {
        slot: slot(0),
        box_id: crusher,
    }));
}

#[test]
fn a_worker_in_the_air_cannot_hit() {
    let mut game = new_game(8, 8, 1);
    place_worker(&mut game, 0, 3, 0);
    send(&mut game, 0, Command::Jump);
    steps(&mut game, 1);
    assert_eq!(worker(&game, 0).state, WorkerState::Falling);
    // On the way down: the second jump is ignored and the crate falls into the head.
    put_falling_box(&mut game, 3, 3);
    send(&mut game, 0, Command::Jump);
    steps(&mut game, 1);

    assert!(broken_events(&game).is_empty());
    assert_eq!(worker(&game, 0).feet, Cell::new(3, 0));
    steps(&mut game, 1);
    assert_eq!(worker(&game, 0).status, WorkerStatus::Crushed);
}

#[test]
fn a_crate_is_not_pushed_over_a_head_even_when_that_worker_jumps_in_the_same_tick() {
    let mut game = new_game(8, 8, 2);
    // Seat 0 tries to push a resting crate over seat 1's head; seat 1 jumps later in the tick.
    put_box(&mut game, 2, 0);
    put_box(&mut game, 2, 1);
    put_box(&mut game, 3, 0);
    put_box(&mut game, 3, 1);
    let pushed = put_box(&mut game, 3, 2);
    place_worker(&mut game, 0, 2, 2);
    place_worker(&mut game, 1, 4, 0);

    send(&mut game, 0, Command::Move(Direction::Right));
    send(&mut game, 1, Command::Jump);
    steps(&mut game, 1);

    assert!(broken_events(&game).is_empty());
    assert_eq!(box_cell(&game, pushed), Some(Cell::new(3, 2)));
    assert!(
        view(&game)
            .events
            .contains(&GameEvent::WorkerJumped { slot: slot(1) })
    );
    assert_eq!(worker(&game, 1).status, WorkerStatus::Alive);
}

#[test]
fn a_crate_still_on_the_crane_is_dropped_onto_the_risen_head() {
    let mut game = new_game(8, 8, 1);
    for y in 0..5 {
        put_box(&mut game, 3, y);
    }
    place_worker(&mut game, 0, 3, 5);
    // Already over the column and done holding: it lets go this tick.
    add_crane(&mut game, 3, 3);

    // The head rises into the top row just before the crane releases its crate there.
    send(&mut game, 0, Command::Jump);
    steps(&mut game, 1);

    assert!(broken_events(&game).is_empty());
    assert_eq!(worker(&game, 0).status, WorkerStatus::Crushed);
    assert_eq!(view(&game).end_reason, Some(EndReason::WorkersCrushed));
}

#[test]
fn workers_jumping_into_the_same_crate_break_it_once_in_seat_order() {
    let mut game = new_game(8, 8, 2);
    place_worker(&mut game, 0, 3, 0);
    place_worker(&mut game, 1, 3, 0);
    let target = put_falling_box(&mut game, 3, 2);

    send(&mut game, 0, Command::Jump);
    send(&mut game, 1, Command::Jump);
    steps(&mut game, 1);

    assert_eq!(
        broken_events(&game),
        vec![GameEvent::BoxBroken {
            box_id: target,
            cell: Cell::new(3, 2),
            by: slot(0),
        }]
    );
    for seat in 0..2 {
        assert_eq!(worker(&game, seat).feet, Cell::new(3, 1));
        assert_eq!(worker(&game, seat).status, WorkerStatus::Alive);
    }
}

#[test]
fn a_head_hit_shields_a_worker_standing_below() {
    let mut game = new_game(8, 8, 2);
    place_worker(&mut game, 0, 3, 0);
    place_worker(&mut game, 1, 3, 0);
    let target = put_falling_box(&mut game, 3, 2);

    send(&mut game, 1, Command::Jump);
    steps(&mut game, 1);

    assert_eq!(
        broken_events(&game),
        vec![GameEvent::BoxBroken {
            box_id: target,
            cell: Cell::new(3, 2),
            by: slot(1),
        }]
    );
    assert_eq!(worker(&game, 0).status, WorkerStatus::Alive);
}

#[test]
fn a_broken_crate_never_completes_a_row() {
    let setup = || {
        // The second worker stands free on top, so the pit alone does not end the game.
        let mut game = new_game(8, 8, 2);
        for y in 0..3 {
            fill_row(&mut game, y, &[3]);
        }
        place_worker(&mut game, 0, 3, 0);
        place_worker(&mut game, 1, 6, 3);
        put_falling_box(&mut game, 3, 3);
        game
    };

    // Standing still, the crate drops through row 2 without completing it and crushes them.
    let mut standing = setup();
    steps(&mut standing, 1);
    assert_eq!(view(&standing).lines_cleared, 0);
    assert_eq!(worker(&standing, 0).status, WorkerStatus::Alive);
    steps(&mut standing, 1);
    assert_eq!(view(&standing).lines_cleared, 0);
    assert_eq!(worker(&standing, 0).status, WorkerStatus::Crushed);

    let mut jumping = setup();
    send(&mut jumping, 0, Command::Jump);
    steps(&mut jumping, 1);
    assert_eq!(broken_events(&jumping).len(), 1);
    assert_eq!(view(&jumping).lines_cleared, 0);
    assert_eq!(view(&jumping).score.get(), 0);
}
