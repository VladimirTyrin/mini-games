//! Word Hunt: guess a hidden word in a few tries; each guess marks its letters as in place,
//! elsewhere in the word, or absent. Russian and English, single player.

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

pub use bots::{BOT_BUDGET, BotKind, GuessingBot};
pub use game::WordHunt;
pub use io::{Command, WorldEvent};
pub use model::{HiddenWord, LetterMark, Phase, ScoredGuess, Word, WordError};
pub use rules::{GROUP_GUESS_CAP, HardModeViolation, Refusal};
pub use settings::{
    DAILY_WORD_LENGTH, DayNumber, FirstTurn, Language, MaxGuesses, Puzzle, PuzzleMode, RawSettings,
    Settings, WordLength,
};
pub use view::{KeyHint, View};

#[cfg(test)]
mod tests;
