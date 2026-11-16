//! Snake for the deterministic game core: rules ([`Snake`]), settings, render model and the
//! built-in bots.
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::unreachable,
        clippy::todo
    )
)]

mod advance;
mod bot_kind;
mod bot_moves;
mod cell_content;
mod command;
mod dead_snake_behavior;
mod death_reason;
mod direction;
mod efficient_bot;
mod field;
mod game_end;
mod grid;
mod move_plan;
mod point;
mod random_bot;
mod score;
mod seat_state;
mod settings;
mod snake;
mod snake_view;
mod start_positions;
mod step_count;
mod view;
mod wall_collision_mode;
mod world_event;

pub use bot_kind::BotKind;
pub use command::{Command, INPUT_BUFFER};
pub use dead_snake_behavior::DeadSnakeBehavior;
pub use death_reason::DeathReason;
pub use direction::Direction;
pub use game_end::GameEnd;
pub use point::Point;
pub use score::Score;
pub use settings::{
    MAX_FIELD_SIZE, MAX_FOOD, MAX_FOOD_PERMILLE, MAX_SEATS, MAX_TICK_MS, MIN_FIELD_SIZE, MIN_FOOD,
    MIN_FOOD_PERMILLE, MIN_SEATS, MIN_TICK_MS, Settings,
};
pub use snake::Snake;
pub use snake_view::SnakeView;
pub use start_positions::START_LENGTH;
pub use step_count::StepCount;
pub use view::View;
pub use wall_collision_mode::WallCollisionMode;
pub use world_event::WorldEvent;

#[cfg(test)]
mod tests;
