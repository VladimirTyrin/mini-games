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
//! 2048 on the deterministic game core: one human slides tiles; equal tiles merge and add their
//! new value to the score; a 2 (90 %) or 4 (10 %) spawns after every move that changed the board
//! (two at the start). Won when a tile reaches the target, lost when no move changes the board.
//!
//! Spawns are randomness resolved by the authority ([`WorldEvent::TileSpawned`]). The [`View`]
//! carries stable tile ids and per-tile [`TileOrigin`]s so clients animate moves without their
//! own copy of the rules.

mod game;
mod io;
mod model;
mod rules;
mod settings;
mod view;

pub use game::Puzzle2048;
pub use io::{Command, WorldEvent};
pub use model::{Cell, Direction, Score, SpawnValue, Tile, TileId, TileValue};
pub use rules::Phase;
pub use settings::{Settings, Target};
pub use view::{MergeSource, TileOrigin, View};

#[cfg(test)]
mod tests;
