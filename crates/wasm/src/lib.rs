//! WebAssembly surface of the game core. Everything crosses the boundary as binary protobuf
//! (`minigames.v2` messages) so the TypeScript side keeps using its generated types.

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

mod catalog;
mod local_host;
mod match_header;
mod match_mirror;
mod match_result;

pub use catalog::{bot_kinds_delimited, default_settings, game_catalog_json, validate_settings};
pub use local_host::LocalHost;
pub use match_mirror::MatchMirror;
