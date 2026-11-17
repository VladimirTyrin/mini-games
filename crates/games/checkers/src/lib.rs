//! Russian draughts (шашки) for two seats: 8×8 board, men capture backwards too, flying kings,
//! compulsory capture (any sequence), draw by threefold repetition or fifteen king-only moves
//! each. See `rules::move_generator` for the moves and `docs/checkers.md` for the rules.

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

pub use bots::{BotKind, EASY_BOT_BUDGET, HARD_BOT_BUDGET, MEDIUM_BOT_BUDGET};
pub use game::Checkers;
pub use io::{Command, WorldEvent};
pub use model::{Path, Piece, PieceId, Rank, Side, Square};
pub use rules::{
    DrawReason, GameEnd, KING_MOVES_FOR_DRAW, LegalMove, REPETITIONS_FOR_DRAW, WinReason,
};
pub use settings::{PLAYERS, RawSettings, Settings, WhitePlayer};
pub use view::{PlacedPiece, View};

#[cfg(test)]
mod tests;
