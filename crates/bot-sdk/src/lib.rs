//! Network bots for the v2 WebSocket protocol (`docs/protocol.md`, `docs/bots.md`).
//!
//! A [`BotSession`] signs in with a bot key, hosts or joins a lobby and plays matches: it mirrors
//! each match with the same core the server runs (lockstep) and asks a [`Strategy`] for commands.
//! Strategies either read the rendered view ([`ViewStrategy`] via [`ViewBot`]) or reuse a
//! built-in core bot ([`BuiltinBot`]). Dropped connections are resumed with the resume token and
//! the match is rebuilt from the server's catch-up.

mod bot_config;
mod bot_session;
mod builtin_bot;
mod connection;
mod lobby_setup;
mod match_play;
mod match_summary;
mod reconnect_policy;
mod registry_mirror;
mod resumption;
mod sdk_error;
mod server_error;
mod strategy;
mod submission;
mod typed_mirror;
mod view_bot;
mod view_strategy;

pub use bot_config::BotConfig;
pub use bot_session::BotSession;
pub use builtin_bot::BuiltinBot;
pub use lobby_setup::LobbySetup;
pub use match_summary::MatchSummary;
pub use reconnect_policy::ReconnectPolicy;
pub use registry_mirror::RegistryMirror;
pub use resumption::Resumption;
pub use sdk_error::SdkError;
pub use server_error::ServerError;
pub use strategy::Strategy;
pub use typed_mirror::TypedMirror;
pub use view_bot::ViewBot;
pub use view_strategy::ViewStrategy;

/// The protocol version this SDK speaks (`Hello.protocol_version`).
pub const PROTOCOL_VERSION: u32 = 2;
