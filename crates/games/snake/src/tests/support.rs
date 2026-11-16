use mg_core::{Game, PlayerSpec, Slot, Viewer};

use crate::cell_content::CellContent;
use crate::{
    DeadSnakeBehavior, Direction, Point, Settings, Snake, SnakeView, WallCollisionMode, WorldEvent,
};

pub(super) fn point(x: u16, y: u16) -> Point {
    Point::new(x, y)
}

pub(super) fn field_settings(
    width: u32,
    height: u32,
    wall_collision_mode: WallCollisionMode,
    dead_snake_behavior: DeadSnakeBehavior,
) -> Settings {
    Settings {
        field_width: width,
        field_height: height,
        wall_collision_mode,
        dead_snake_behavior,
        ..Settings::default()
    }
}

pub(super) fn new_game(settings: Settings, players: &[PlayerSpec]) -> Snake {
    Snake::new(&settings, players).unwrap()
}

pub(super) fn humans(count: usize) -> Vec<PlayerSpec> {
    vec![PlayerSpec::HUMAN; count]
}

/// `length` cells starting at `head` and extending towards `tail_side`.
pub(super) fn line(head: Point, tail_side: Direction, length: u16) -> Vec<Point> {
    (0..length)
        .map(|offset| match tail_side {
            Direction::Up => point(head.x, head.y - offset),
            Direction::Down => point(head.x, head.y + offset),
            Direction::Left => point(head.x - offset, head.y),
            Direction::Right => point(head.x + offset, head.y),
        })
        .collect()
}

/// Repositions every seat (head-first bodies); cells must not overlap.
pub(super) fn arrange(game: &mut Snake, layout: &[(Vec<Point>, Direction)]) {
    assert_eq!(layout.len(), game.seats.len(), "arrange needs every seat");
    for seat in &game.seats {
        for &cell in &seat.body {
            game.grid.set(cell, CellContent::Empty);
        }
    }
    for ((slot, seat), (body, direction)) in Slot::all(layout.len())
        .zip(game.seats.iter_mut())
        .zip(layout)
    {
        for &cell in body {
            assert!(game.grid.is_empty_cell(cell), "{cell:?} is taken");
            game.grid.set(cell, CellContent::Body(slot));
        }
        seat.body = body.iter().copied().collect();
        seat.direction = *direction;
        seat.queued_turns.clear();
    }
}

pub(super) fn put_food(game: &mut Snake, cell: Point) {
    game.apply_world(&WorldEvent::SpawnFood(cell)).unwrap();
}

pub(super) fn snake(game: &Snake, slot: u8) -> SnakeView {
    game.view(Viewer::Spectator).snakes[usize::from(slot)].clone()
}
