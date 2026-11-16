//! Deterministic game core shared by the server, the browser (WASM), the Android app, bots and
//! the arena. Lockstep model: an [`Authority`] seals [`Frame`]s, receivers replay them on a
//! [`Match`]. See `docs/architecture.md` and `docs/adding-a-game.md`.

mod authority;
mod awaiting;
mod bot;
mod bot_seat;
mod budget;
mod clock;
mod command_error;
mod compact_log;
mod frame;
mod frame_error;
mod game;
mod game_info;
mod has_bots;
mod hash;
mod hash_policy;
mod matcher;
mod outcome;
mod player;
mod rng;
mod setup_error;
mod slot;
mod tick;
mod viewer;

#[cfg(any(test, feature = "conformance"))]
pub mod conformance;

pub use authority::Authority;
pub use awaiting::Awaiting;
pub use bot::Bot;
pub use bot_seat::BotSeat;
pub use budget::Budget;
pub use clock::Clock;
pub use command_error::CommandError;
pub use compact_log::expand_compact_log;
pub use frame::{Frame, GameFrame, GameInput, Input};
pub use frame_error::FrameError;
pub use game::Game;
pub use game_info::GameInfo;
pub use has_bots::HasBots;
pub use hash::StateHasher;
pub use hash_policy::HashPolicy;
pub use matcher::Match;
pub use outcome::{Outcome, OutcomeKind, Status};
pub use player::{PlayerKind, PlayerSpec};
pub use rng::CoreRng;
pub use setup_error::SetupError;
pub use slot::Slot;
pub use tick::Tick;
pub use viewer::Viewer;

#[cfg(test)]
mod tests;
