mod bot_kind;
mod easy_bot;
mod evaluation;
mod search_bot;

pub use bot_kind::{BotKind, EASY_BOT_BUDGET, HARD_BOT_BUDGET, MEDIUM_BOT_BUDGET};
pub(crate) use easy_bot::EasyBot;
pub(crate) use search_bot::SearchBot;
