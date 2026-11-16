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

mod game;
mod io;
mod model;
mod rules;
mod settings;
mod view;

pub use game::NumbersMatch;
pub use io::{Command, WorldEvent};
pub use model::{Cell, CellIndex, CellPair, CellValue, Hint, HintsLeft, RowIndex};
pub use rules::{COLUMNS, INITIAL_CELLS, LossReason, Phase};
pub use settings::{
    DEFAULT_REFILLS, HintMode, LIMITED_HINTS_PER_REFILL, LIMITED_INITIAL_HINTS, MAX_REFILLS,
    Settings,
};
pub use view::{LastAction, Stats, View};

#[cfg(test)]
mod tests;
