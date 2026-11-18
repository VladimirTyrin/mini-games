//! Where crashes are shown: in half cells, a cell `x` spanning `2x..=2x+2`.

use mg_core::{Game, Slot, Viewer};

use super::support::{arrange, field_settings, humans, line, new_game, point};
use crate::{Crash, CrashPoint, DeadSnakeBehavior, Direction, Snake, WallCollisionMode};

use Direction::{Down, Left, Right, Up};
use WallCollisionMode::{Death, WrapAround};

fn game(seats: usize, wall: WallCollisionMode) -> Snake {
    new_game(
        field_settings(20, 20, wall, DeadSnakeBehavior::StayOnField),
        &humans(seats),
    )
}

fn crashes(game: &Snake) -> Vec<Crash> {
    game.view(Viewer::Spectator).crashes
}

fn crash(x2: u32, y2: u32, slots: &[u8], head_on: bool) -> Crash {
    Crash {
        at: CrashPoint { x2, y2 },
        slots: slots.iter().map(|&slot| Slot::new(slot)).collect(),
        head_on,
    }
}

#[test]
fn heads_entering_one_cell_crash_in_its_middle() {
    let mut game = game(3, Death);
    arrange(
        &mut game,
        &[
            (line(point(4, 5), Left, 3), Right),
            (line(point(6, 5), Right, 3), Left),
            (line(point(15, 15), Down, 3), Up),
        ],
    );
    game.step();
    assert_eq!(crashes(&game), [crash(11, 11, &[0, 1], true)]);
}

#[test]
fn heads_swapping_cells_crash_on_the_edge_between_them() {
    let mut game = game(2, Death);
    arrange(
        &mut game,
        &[
            (line(point(4, 5), Left, 3), Right),
            (line(point(5, 5), Right, 3), Left),
        ],
    );
    game.step();
    assert_eq!(crashes(&game), [crash(10, 11, &[0, 1], true)]);
}

#[test]
fn heads_swapping_across_a_wrap_around_border_meet_on_it() {
    let mut game = game(2, WrapAround);
    arrange(
        &mut game,
        &[
            (line(point(19, 5), Left, 3), Right),
            (line(point(0, 5), Right, 3), Left),
        ],
    );
    game.step();
    assert_eq!(crashes(&game), [crash(0, 11, &[0, 1], true)]);
}

#[test]
fn a_wall_crash_is_on_the_field_edge_in_front_of_the_head() {
    let mut game = game(2, Death);
    arrange(
        &mut game,
        &[
            (line(point(19, 5), Left, 3), Right),
            (line(point(10, 15), Down, 3), Up),
        ],
    );
    game.step();
    assert_eq!(crashes(&game), [crash(40, 11, &[0], false)]);
}

#[test]
fn snakes_still_alive_show_no_crash() {
    let mut game = game(2, Death);
    arrange(
        &mut game,
        &[
            (line(point(4, 5), Left, 3), Right),
            (line(point(10, 15), Down, 3), Up),
        ],
    );
    game.step();
    assert!(crashes(&game).is_empty());
}
