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
}
