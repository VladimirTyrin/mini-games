//! Local bot tournaments. Every match runs on the same registry authority the server uses, so a
//! result here is what the bots would do online. Results depend only on the base seed: each game
//! derives its own seed from it, whatever the thread count.

mod arena_bot;
mod arena_config;
mod arena_error;
mod arena_report;
mod game_lookup;
mod game_seed;
mod played_game;
mod replay_writer;
mod run_arena;
mod seat_tally;

pub use arena_bot::ArenaBot;
pub use arena_config::ArenaConfig;
pub use arena_error::ArenaError;
pub use arena_report::ArenaReport;
pub use game_lookup::{game_by_id, settings_from_json};
pub use game_seed::game_seed;
pub use run_arena::run_arena;
pub use seat_tally::SeatTally;
