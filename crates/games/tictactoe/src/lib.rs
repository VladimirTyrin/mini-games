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

mod bots;
mod game;
mod io;
mod model;
mod rules;
mod settings;
mod view;

pub use bots::{BotKind, MINIMAX_BOT_BUDGET, MinimaxBot, RANDOM_BOT_BUDGET, RandomBot};
pub use game::TicTacToe;
pub use io::{Command, WorldEvent};
pub use model::{Mark, Point};
pub use rules::{GameEnd, WinningLine};
pub use settings::{FieldSize, FirstPlayerMode, PLAYERS, RawSettings, Settings};
pub use view::View;

#[cfg(test)]
mod tests;
