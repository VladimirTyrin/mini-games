use mg_core::{
    Awaiting, Clock, Game, GameSettings, OutOfRange, PlayerSpec, SettingProblem, SettingsError,
    SetupError, Status, Viewer,
};

use super::support::{field_settings, humans, new_game, point};
use crate::{
    DeadSnakeBehavior, Direction, MAX_SEATS, RawSettings, START_LENGTH, Settings, Snake,
    WallCollisionMode,
};

#[test]
fn defaults_match_the_web_client() {
    assert_eq!(
        RawSettings::default(),
        RawSettings {
            field_width: 15,
            field_height: 15,
            wall_collision_mode: WallCollisionMode::WrapAround,
            dead_snake_behavior: DeadSnakeBehavior::Disappear,
            max_food_count: 1,
            food_spawn_permille: 1000,
            tick_interval_ms: 200,
        }
    );
}

#[test]
fn out_of_range_settings_name_the_field() {
    let settings = RawSettings {
        food_spawn_permille: 1001,
        ..RawSettings::default()
    };
    assert_eq!(
        Settings::parse(settings),
        Err(SettingsError {
            field: "food_spawn_permille",
            problem: SettingProblem::OutOfRange(OutOfRange {
                value: 1001,
                min: 1,
                max: 1000
            }),
        })
    );
    let error = Snake::new(&Settings::default(), &humans(11)).unwrap_err();
    assert!(matches!(error, SetupError::PlayerCount { got: 11, .. }));
}

#[test]
fn start_positions_never_overlap() {
    for seats in 1..=usize::from(MAX_SEATS) {
        for width in 10..=100 {
            for height in [10, 11, 57, 100] {
                let settings = field_settings(
                    width,
                    height,
                    WallCollisionMode::Death,
                    DeadSnakeBehavior::Disappear,
                );
                let view = new_game(settings, &humans(seats)).view(Viewer::Spectator);
                let mut cells: Vec<_> = view
                    .snakes
                    .iter()
                    .flat_map(|snake| snake.body.iter().copied())
                    .collect();
                assert!(view.snakes.iter().all(|snake| {
                    snake.body.len() == usize::from(START_LENGTH)
                        && snake.direction == Direction::Up
                        && snake.is_alive()
                }));
                assert!(
                    cells
                        .iter()
                        .all(|cell| u32::from(cell.x) < width && u32::from(cell.y) < height)
                );
                cells.sort();
                cells.dedup();
                assert_eq!(
                    cells.len(),
                    seats * usize::from(START_LENGTH),
                    "{seats} seats on {width}x{height}"
                );
            }
        }
    }
}

#[test]
fn ten_seats_on_the_narrowest_field_get_one_column_each() {
    let settings = field_settings(
        10,
        10,
        WallCollisionMode::Death,
        DeadSnakeBehavior::Disappear,
    );
    let view = new_game(settings, &humans(10)).view(Viewer::Spectator);
    let heads: Vec<_> = view.snakes.iter().map(|snake| snake.body[0]).collect();
    let expected: Vec<_> = (0..10).map(|column| point(column, 5)).collect();
    assert_eq!(heads, expected);
}

#[test]
fn fresh_game_is_realtime_and_waits_for_everyone() {
    let game = new_game(
        RawSettings::default(),
        &[PlayerSpec::HUMAN, PlayerSpec::BOT],
    );
    assert_eq!(game.clock(), Clock::Realtime { tick_ms: 200 });
    assert_eq!(game.awaiting(), Awaiting::Simultaneous);
    assert_eq!(game.status(), Status::InProgress);
    let view = game.view(Viewer::Player(mg_core::Slot::new(1)));
    assert_eq!(view.you, Some(mg_core::Slot::new(1)));
    assert!(view.food.is_empty());
    assert_eq!(view.steps, crate::StepCount::ZERO);
}
