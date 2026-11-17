mod bot_kind;
mod density;
mod hunter_bot;

pub use bot_kind::{BOT_BUDGET, BotKind};
pub(crate) use density::ship_density;
pub use hunter_bot::HunterBot;
