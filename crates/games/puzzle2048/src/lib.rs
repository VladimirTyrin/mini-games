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

mod board;
mod cell;
mod command;
mod direction;
mod merge_source;
mod phase;
mod puzzle2048;
mod score;
mod settings;
mod slide_outcome;
mod spawn_value;
mod target;
mod tile;
mod tile_id;
mod tile_origin;
mod tile_value;
mod view;
mod world_event;

pub use cell::Cell;
pub use command::Command;
pub use direction::Direction;
pub use merge_source::MergeSource;
pub use phase::Phase;
pub use puzzle2048::Puzzle2048;
pub use score::Score;
pub use settings::Settings;
pub use spawn_value::SpawnValue;
pub use target::Target;
pub use tile::Tile;
pub use tile_id::TileId;
pub use tile_origin::TileOrigin;
pub use tile_value::TileValue;
pub use view::View;
pub use world_event::WorldEvent;

#[cfg(test)]
mod tests;
