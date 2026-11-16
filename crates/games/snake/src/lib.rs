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

mod bots;
mod game;
mod io;
mod model;
mod rules;
mod settings;
mod view;

pub use bots::BotKind;
pub use game::Snake;
pub use io::{Command, INPUT_BUFFER, WorldEvent};
pub use model::{Direction, Point, Score, StepCount};
pub use rules::{DeathReason, GameEnd, START_LENGTH};
pub use settings::{
    DeadSnakeBehavior, MAX_FIELD_SIZE, MAX_FOOD, MAX_FOOD_PERMILLE, MAX_SEATS, MAX_TICK_MS,
    MIN_FIELD_SIZE, MIN_FOOD, MIN_FOOD_PERMILLE, MIN_SEATS, MIN_TICK_MS, Settings,
    WallCollisionMode,
};
pub use view::{SnakeView, View};

#[cfg(test)]
mod tests;
