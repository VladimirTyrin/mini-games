use mg_core::Slot;

use crate::model::{Phase, ScoredGuess};
use crate::settings::Settings;
use crate::view::KeyHint;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub settings: Settings,
    pub phase: Phase,
    pub guesses: Vec<ScoredGuess>,
    /// Letters guessed so far with their best mark, in alphabet order.
    pub keyboard: Vec<KeyHint>,
    /// Shown once the game is over.
    pub answer: Option<String>,
    /// Whose guess is next; `None` before the start and after the end.
    pub turn: Option<Slot>,
    /// Group games: who found the word (or outlasted the others).
    pub winner: Option<Slot>,
    /// Guesses the game allows in total (the setting alone, the safety cap in a group).
    pub guess_limit: u32,
}
