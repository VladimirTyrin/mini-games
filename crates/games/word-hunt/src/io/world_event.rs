use mg_core::Slot;

use crate::model::LetterMark;

/// What the authority resolves: chance, and what only it knows (the word).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldEvent {
    /// The word is chosen. It stays with the authority until the game is over.
    WordChosen,
    /// Group games: who guesses first.
    FirstPlayerChosen(Slot),
    /// The marks of the guess just made, position by position.
    GuessMarked(Vec<LetterMark>),
    /// The word, once the game is over. Carrying the word itself (not an index into the list)
    /// keeps replays valid when the answer list changes.
    WordRevealed(String),
}
