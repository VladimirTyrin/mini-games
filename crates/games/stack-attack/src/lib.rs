//! Stack Attack: cooperative real-time survival. Cranes drop boxes onto the field; workers push
//! and climb them to complete rows. The game ends when every worker is crushed or gone, or when a
//! resting box reaches the top row. Rule changes against the legacy server are listed in
//! `docs/refactoring-plan.md`, section 7.
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

mod box_id;
mod cargo_box;
mod cell;
mod command;
mod crane;
mod crane_id;
mod crane_state;
mod direction;
mod end_reason;
mod field;
mod game_event;
mod level;
mod pattern_id;
pub mod rules;
mod score;
mod settings;
mod side;
mod simulation;
mod stack_attack;
mod view;
mod worker;
mod worker_state;
mod worker_status;
mod worker_view;
mod world_event;

pub use box_id::BoxId;
pub use cargo_box::CargoBox;
pub use cell::Cell;
pub use command::{Command, MAX_QUEUED_COMMANDS};
pub use crane::Crane;
pub use crane_id::CraneId;
pub use crane_state::CraneState;
pub use direction::Direction;
pub use end_reason::EndReason;
pub use game_event::GameEvent;
pub use level::Level;
pub use pattern_id::PatternId;
pub use score::Score;
pub use settings::{
    MAX_FIELD_HEIGHT, MAX_FIELD_WIDTH, MAX_SEATS, MIN_FIELD_HEIGHT, MIN_FIELD_WIDTH, MIN_SEATS,
    Settings,
};
pub use side::Side;
pub use stack_attack::StackAttack;
pub use view::View;
pub use worker_state::WorkerState;
pub use worker_status::WorkerStatus;
pub use worker_view::WorkerView;
pub use world_event::WorldEvent;

#[cfg(test)]
mod tests;
