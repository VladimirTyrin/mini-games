use mg_core::conformance::Conformance;
use mg_core::{Game, Slot};

use super::support::{arrange, field_settings, humans, line, new_game, point, put_food, snake};
use crate::{Command, DeadSnakeBehavior, DeathReason, Direction, Snake, WallCollisionMode};

use DeadSnakeBehavior::{Disappear, StayOnField};
use Direction::{Down, Left, Right, Up};
use WallCollisionMode::{Death, WrapAround};

fn game(seats: usize, wall: WallCollisionMode, dead: DeadSnakeBehavior) -> Snake {
    new_game(field_settings(20, 20, wall, dead), &humans(seats))
}

fn death(game: &Snake, slot: u8) -> Option<DeathReason> {
    snake(game, slot).death
}

#[test]
fn snakes_move_one_cell_per_tick() {
    let mut game = game(1, Death, Disappear);
    let before = snake(&game, 0).body;
    game.step();
    let after = snake(&game, 0).body;
    assert_eq!(after[0], point(before[0].x, before[0].y - 1));
    assert_eq!(after[1..], before[..2]);
    assert_eq!(game.view(mg_core::Viewer::Spectator).tick, 1);
}

#[test]
fn walls_kill_or_wrap() {
    let mut deadly = game(2, Death, Disappear);
    arrange(
        &mut deadly,
        &[
            (line(point(3, 0), Down, 3), Up),
            (line(point(10, 10), Down, 3), Up),
        ],
    );
    deadly.step();
    assert_eq!(death(&deadly, 0), Some(DeathReason::WallCollision));

    let mut wrapping = game(2, WrapAround, Disappear);
    arrange(
        &mut wrapping,
        &[
            (line(point(3, 0), Down, 3), Up),
            (line(point(19, 10), Left, 3), Right),
        ],
    );
    wrapping.step();
    assert_eq!(snake(&wrapping, 0).body[0], point(3, 19));
    assert_eq!(snake(&wrapping, 1).body[0], point(0, 10));
    assert!(snake(&wrapping, 0).is_alive() && snake(&wrapping, 1).is_alive());
}

#[test]
fn eating_grows_and_scores() {
    let mut game = game(1, Death, Disappear);
    let head = snake(&game, 0).body[0];
    put_food(&mut game, point(head.x, head.y - 1));
    game.step();
    let fed = snake(&game, 0);
    assert_eq!(fed.body.len(), 4);
    assert_eq!(fed.score.get(), 1);
    assert!(game.food.is_empty());
}

#[test]
fn running_into_own_body_is_self_collision() {
    let mut game = game(2, Death, Disappear);
    let hook = vec![
        point(5, 5),
        point(6, 5),
        point(6, 6),
        point(5, 6),
        point(4, 6),
    ];
    arrange(
        &mut game,
        &[(hook, Left), (line(point(15, 15), Down, 3), Up)],
    );
    game.apply_command(Slot::new(0), &Command::Turn(Down))
        .unwrap();
    game.step();
    assert_eq!(death(&game, 0), Some(DeathReason::SelfCollision));
}

#[test]
fn heads_entering_the_same_cell_both_die() {
    let mut game = game(3, Death, Disappear);
    arrange(
        &mut game,
        &[
            (line(point(4, 5), Left, 3), Right),
            (line(point(6, 5), Right, 3), Left),
            (line(point(15, 15), Down, 3), Up),
        ],
    );
    put_food(&mut game, point(5, 5));
    game.step();
    assert_eq!(death(&game, 0), Some(DeathReason::SnakeCollision));
    assert_eq!(death(&game, 1), Some(DeathReason::SnakeCollision));
    assert_eq!(game.food, [point(5, 5)], "nobody eats in a head-on crash");
}

#[test]
fn swapping_cells_kills_both() {
    let mut game = game(3, Death, Disappear);
    arrange(
        &mut game,
        &[
            (line(point(5, 5), Left, 3), Right),
            (line(point(6, 5), Right, 3), Left),
            (line(point(15, 15), Down, 3), Up),
        ],
    );
    game.step();
    assert_eq!(death(&game, 0), Some(DeathReason::SnakeCollision));
    assert_eq!(death(&game, 1), Some(DeathReason::SnakeCollision));
    assert!(snake(&game, 2).is_alive());
}

#[test]
fn entering_a_tail_vacated_this_tick_is_allowed() {
    let mut game = game(2, Death, Disappear);
    arrange(
        &mut game,
        &[
            (line(point(5, 5), Left, 3), Right),
            (line(point(6, 3), Down, 3), Up),
        ],
    );
    game.step();
    assert!(snake(&game, 0).is_alive() && snake(&game, 1).is_alive());
    assert_eq!(snake(&game, 0).body[0], point(6, 5));
}

#[test]
fn entering_the_tail_of_a_growing_snake_kills() {
    let mut game = game(2, Death, Disappear);
    arrange(
        &mut game,
        &[
            (line(point(5, 5), Left, 3), Right),
            (line(point(6, 3), Down, 3), Up),
        ],
    );
    put_food(&mut game, point(6, 2));
    game.step();
    assert_eq!(death(&game, 0), Some(DeathReason::SnakeCollision));
    assert_eq!(snake(&game, 1).score.get(), 1);
}

#[test]
fn entering_the_tail_of_a_dying_snake_kills() {
    let mut game = game(3, Death, Disappear);
    arrange(
        &mut game,
        &[
            (line(point(5, 2), Left, 3), Right),
            (line(point(6, 0), Down, 3), Up),
            (line(point(15, 15), Down, 3), Up),
        ],
    );
    game.step();
    assert_eq!(death(&game, 1), Some(DeathReason::WallCollision));
    assert_eq!(death(&game, 0), Some(DeathReason::SnakeCollision));
}

#[test]
fn snakes_chasing_each_others_tails_in_a_ring_survive() {
    let mut game = new_game(field_settings(10, 10, WrapAround, Disappear), &humans(2));
    arrange(
        &mut game,
        &[
            (line(point(4, 0), Left, 5), Right),
            (line(point(9, 0), Left, 5), Right),
        ],
    );
    for _ in 0..25 {
        game.step();
        game.check_invariants().unwrap();
    }
    assert!(snake(&game, 0).is_alive() && snake(&game, 1).is_alive());
}

/// Legacy removed the head from its occupancy set when the head entered the own tail cell.
#[test]
fn chasing_own_tail_keeps_the_state_consistent() {
    let mut game = game(2, Death, Disappear);
    let square = vec![point(1, 1), point(2, 1), point(2, 2), point(1, 2)];
    arrange(
        &mut game,
        &[(square, Left), (line(point(15, 15), Down, 3), Up)],
    );
    for direction in [Down, Right, Up, Left].into_iter().cycle().take(12) {
        game.apply_command(Slot::new(0), &Command::Turn(direction))
            .unwrap();
        game.step();
        game.check_invariants().unwrap();
        assert!(snake(&game, 0).is_alive());
    }
    // The square is still solid: the other snake cannot pass through it.
    let looping = snake(&game, 0);
    arrange(
        &mut game,
        &[
            (looping.body, looping.direction),
            (line(point(3, 1), Right, 3), Left),
        ],
    );
    game.step();
    assert_eq!(death(&game, 1), Some(DeathReason::SnakeCollision));
}

#[test]
fn dead_bodies_block_only_with_stay_on_field() {
    for (behavior, survives) in [(StayOnField, false), (Disappear, true)] {
        let mut game = game(3, Death, behavior);
        arrange(
            &mut game,
            &[
                (line(point(6, 6), Down, 3), Up),
                (line(point(5, 7), Left, 3), Right),
                (line(point(15, 15), Down, 3), Up),
            ],
        );
        game.drop_player(Slot::new(0));
        game.step();
        assert_eq!(snake(&game, 1).is_alive(), survives, "{behavior:?}");
        game.check_invariants().unwrap();
    }
}
