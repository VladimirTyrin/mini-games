mod bot_kind;
mod candidate;
mod minimax_bot;
mod neighbourhood;
mod probe;
mod random_bot;
mod root_move;
mod run;
mod score;
mod search;
mod search_board;
mod side;
mod side_probe;
mod stone;
mod tactics;
mod window_counts;

pub use bot_kind::{BotKind, MINIMAX_BOT_BUDGET, RANDOM_BOT_BUDGET};
pub use minimax_bot::MinimaxBot;
pub use random_bot::RandomBot;

#[cfg(test)]
pub(crate) use {
    candidate::Candidate, neighbourhood::Neighbourhood, score::Score, search::Search,
    search_board::SearchBoard, side::Side,
};
