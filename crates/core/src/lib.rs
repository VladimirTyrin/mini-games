//! Deterministic game core shared by the server, the browser (WASM), the Android app, bots and
//! the arena. Lockstep model: an [`Authority`] seals [`Frame`]s, receivers replay them on a
//! [`Match`]. See `docs/architecture.md` and `docs/adding-a-game.md`.

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

mod authority;
mod awaiting;
mod bot;
mod bot_mind;
mod bot_seat;
mod bot_turn;
mod budget;
mod cancel_token;
mod capped;
mod clock;
mod command_error;
mod compact_log;
mod compact_log_entries;
mod compact_log_entry;
mod compact_log_error;
mod decided_bot_turn;
mod frame;
mod frame_error;
mod game;
mod game_info;
mod game_secret;
mod game_settings;
mod has_bots;
mod hash;
mod hash_policy;
mod input;
mod logged_frame;
mod matcher;
mod out_of_range;
mod outcome;
mod outcome_kind;
mod player_kind;
mod player_spec;
mod rng;
mod seat_check;
mod seated_bot;
mod setting_problem;
mod setting_range;
mod settings_error;
mod setup_error;
mod slot;
mod status;
mod tick;
mod viewer;

#[cfg(any(test, feature = "conformance"))]
pub mod conformance;

pub use authority::Authority;
pub use awaiting::Awaiting;
pub use bot::Bot;
pub use bot_seat::BotSeat;
pub use bot_turn::BotTurn;
pub use budget::Budget;
pub use cancel_token::CancelToken;
pub use capped::Capped;
pub use clock::Clock;
pub use command_error::CommandError;
pub use compact_log::CompactLog;
pub use compact_log_entries::CompactLogEntries;
pub use compact_log_entry::CompactLogEntry;
pub use compact_log_error::CompactLogError;
pub use decided_bot_turn::DecidedBotTurn;
pub use frame::{Frame, GameFrame};
pub use frame_error::FrameError;
pub use game::Game;
pub use game_info::GameInfo;
pub use game_secret::{GameSecret, SeatSecret, SecretCommand};
pub use game_settings::GameSettings;
pub use has_bots::HasBots;
pub use hash::StateHasher;
pub use hash_policy::HashPolicy;
pub use input::{GameInput, Input};
pub use logged_frame::LoggedFrame;
pub use matcher::Match;
pub use out_of_range::OutOfRange;
pub use outcome::Outcome;
pub use outcome_kind::OutcomeKind;
pub use player_kind::PlayerKind;
pub use player_spec::PlayerSpec;
pub use rng::CoreRng;
pub use setting_problem::SettingProblem;
pub use setting_range::SettingRange;
pub use settings_error::SettingsError;
pub use setup_error::SetupError;
pub use slot::Slot;
pub use status::Status;
pub use tick::Tick;
pub use viewer::Viewer;

#[cfg(test)]
mod tests;
