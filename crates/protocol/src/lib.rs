//! Wire contract (generated from `proto/`) and conversions between game domain types and it.

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

mod frame_codec;
mod games;
mod logged_wire_frame;
mod outcome_codec;
mod replay_file_error;
mod replay_format;
mod wire_error;
mod wire_game;

pub use frame_codec::{frame_from_wire, frame_to_wire, slot_from_wire, slot_to_wire};
pub use outcome_codec::outcome_to_wire;
pub use replay_file_error::ReplayFileError;
pub use replay_format::{REPLAY_FORMAT_VERSION, decode_replay_file};
pub use wire_error::WireError;
pub use wire_game::{RawSettingsOf, WireBots, WireGame};

/// Generated protobuf types of package `minigames.v2`.
#[allow(clippy::all, missing_debug_implementations, unreachable_pub)]
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/minigames.v2.rs"));
    #[cfg(feature = "json")]
    include!(concat!(env!("OUT_DIR"), "/minigames.v2.serde.rs"));
}
