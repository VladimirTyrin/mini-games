//! Stack Attack: cooperative real-time survival. Cranes drop boxes onto the field; workers push
//! and climb them to complete rows, and break falling ones by jumping into them (a head hit, no
//! points). The game ends when every worker is crushed or gone, or when a
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

mod game;
mod io;
mod model;
pub mod rules;
mod settings;
mod view;

pub use game::StackAttack;
pub use io::{Command, MAX_QUEUED_COMMANDS, WorldEvent};
pub use model::{
    BoxId, CargoBox, Cell, Column, Crane, CraneId, CraneState, Direction, Level, PatternId, Row,
    Score, Side, TickCount, WorkerState, WorkerStatus,
};
pub use rules::EndReason;
pub use settings::{FieldHeight, FieldWidth, MAX_SEATS, MIN_SEATS, RawSettings, Settings};
pub use view::{GameEvent, View, WorkerView};

#[cfg(test)]
mod tests;
