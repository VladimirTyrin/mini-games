//! Wire contract (generated from `proto/`) and conversions between game domain types and it.

mod frame_codec;
mod wire_error;
mod wire_game;

pub use frame_codec::{frame_from_wire, frame_to_wire, slot_from_wire};
pub use wire_error::WireError;
pub use wire_game::{WireBots, WireGame};

/// Generated protobuf types of package `minigames.v2`.
#[allow(clippy::all, missing_debug_implementations, unreachable_pub)]
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/minigames.v2.rs"));
    #[cfg(feature = "json")]
    include!(concat!(env!("OUT_DIR"), "/minigames.v2.serde.rs"));
}
