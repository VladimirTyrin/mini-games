//! Deterministic game core shared by the server, the browser (WASM), the Android app, bots and
//! the arena.
//!
//! * [`Game`] — rules of one game. Pure, deterministic, no IO.
//! * [`Authority`] — runs a match: validates commands, runs bots, resolves randomness, seals
//!   [`Frame`]s.
//! * [`Match`] — applies frames on the receiving side (clients, observers, replays).
//!
//! See `docs/adding-a-game.md` for how to add a game.

mod authority;
mod bot;
mod error;
mod frame;
mod game;
mod hash;
mod matcher;
mod rng;

#[cfg(any(test, feature = "conformance"))]
pub mod conformance;

pub use authority::{Authority, BotSeat, HashPolicy, expand_log};
pub use bot::{Bot, Budget, HasBots};
pub use error::{CommandError, FrameError, SetupError};
pub use frame::{Frame, GameFrame, GameInput, Input};
pub use game::{
    Awaiting, Clock, Game, GameInfo, Outcome, OutcomeKind, PlayerKind, PlayerSpec, Slot, Status,
    Viewer,
};
pub use hash::StateHasher;
pub use matcher::Match;
pub use rng::CoreRng;

#[cfg(test)]
mod tests;
