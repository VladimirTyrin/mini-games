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
//! **Numbers Match**: a single-player number puzzle on a 9-column grid.
//!
//! The authority deals [`INITIAL_CELLS`] digits in the first frame
//! ([`WorldEvent::BoardDealt`]). The player removes pairs of active cells whose values are equal
//! or sum to 10 when every cell between them on a straight line, or in reading order, is already
//! inactive; rows without active cells are deleted. A refill appends a copy of every active value
//! (deterministic, no randomness). Clearing the board wins; no pair and no refills left loses.
//! Cells are addressed by [`CellIndex`] (reading order).

mod board;
mod cell;
mod cell_index;
mod cell_pair;
mod cell_value;
mod command;
mod hint;
mod hint_mode;
mod hints_left;
mod last_action;
mod loss_reason;
mod numbers_match;
mod phase;
mod row_index;
mod settings;
mod stats;
mod view;
mod world_event;

pub use cell::Cell;
pub use cell_index::CellIndex;
pub use cell_pair::CellPair;
pub use cell_value::CellValue;
pub use command::Command;
pub use hint::Hint;
pub use hint_mode::HintMode;
pub use hints_left::HintsLeft;
pub use last_action::LastAction;
pub use loss_reason::LossReason;
pub use numbers_match::NumbersMatch;
pub use phase::Phase;
pub use row_index::RowIndex;
pub use settings::{
    DEFAULT_REFILLS, LIMITED_HINTS_PER_REFILL, LIMITED_INITIAL_HINTS, MAX_REFILLS, Settings,
};
pub use stats::Stats;
pub use view::View;
pub use world_event::WorldEvent;

pub const COLUMNS: usize = 9;
pub const INITIAL_CELLS: usize = 42;

#[cfg(test)]
mod tests;
