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
mod refusal_codec;
mod refusal_english;
mod replay_file_error;
mod replay_format;
mod setting_problem_codec;
mod wire_error;
mod wire_game;
mod wire_refusal;

pub use frame_codec::{frame_from_wire, frame_to_wire, slot_from_wire, slot_to_wire};
pub use outcome_codec::outcome_to_wire;
use refusal_codec::malformed_refusal;
pub use refusal_codec::{common_refusal, refusal_to_wire};
pub use refusal_english::{refusal_english, setting_rule_english};
pub use replay_file_error::ReplayFileError;
pub use replay_format::{REPLAY_FORMAT_VERSION, decode_replay_file};
pub use setting_problem_codec::setting_problem_to_wire;
pub use wire_error::WireError;
pub use wire_game::{RawSettingsOf, WireBots, WireGame};
pub use wire_refusal::WireRefusal;

/// Generated protobuf types of package `minigames.v2`.
#[allow(clippy::all, missing_debug_implementations, unreachable_pub)]
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/minigames.v2.rs"));
    #[cfg(feature = "json")]
    include!(concat!(env!("OUT_DIR"), "/minigames.v2.serde.rs"));
}
