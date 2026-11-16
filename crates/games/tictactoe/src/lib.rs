//! TicTacToe / gomoku: two seats place X and O on a W×H board; `win_count` in a row (any of the
//! four directions) wins, a full board is a draw. Ships a random and a minimax bot.

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

mod board;
mod bots;
mod command;
mod first_player_mode;
mod game_end;
mod line_direction;
mod mark;
mod point;
mod settings;
mod tictactoe;
mod view;
mod winning_line;
mod world_event;

pub use bots::{BotKind, MINIMAX_BOT_BUDGET, MinimaxBot, RANDOM_BOT_BUDGET, RandomBot};
pub use command::Command;
pub use first_player_mode::FirstPlayerMode;
pub use game_end::GameEnd;
pub use mark::Mark;
pub use point::Point;
pub use settings::{MAX_FIELD_SIZE, MIN_FIELD_SIZE, MIN_WIN_COUNT, PLAYERS, Settings};
pub use tictactoe::TicTacToe;
pub use view::View;
pub use winning_line::WinningLine;
pub use world_event::WorldEvent;

#[cfg(test)]
mod tests;
