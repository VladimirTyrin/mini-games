use std::fmt;

use mg_core::EnglishText;

/// Why a guess or giving up is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    WordNotChosenYet,
    LastGuessNotMarked,
    WrongLength,
    NotALetter,
    AlreadyTried,
    /// Hard mode: a letter found in place (green) is missing from its position.
    MovedCorrectLetter,
    /// Hard mode: a letter found elsewhere (yellow), or a found letter's count, is missing.
    DroppedFoundLetter,
    CannotGiveUp,
    // Input no honest client or authority sends: a world event that does not fit the state, a
    // seat or cell that does not exist. Clients show it as a generic "not allowed".
    NoGuessPending,
    MarksFitNoGuess,
    WordAlreadyChosen,
    FirstPlayerAlreadyChosen,
    FirstPlayerAgainstSettings,
    WordRevealedEarly,
    RevealedWordInvalid,
}

impl EnglishText for Refusal {
    fn english(self) -> &'static str {
        match self {
            Self::WordNotChosenYet => "the word is not chosen yet",
            Self::LastGuessNotMarked => "the last guess is not marked yet",
            Self::WrongLength => "the word has the wrong number of letters",
            Self::NotALetter => "the word has a character outside the alphabet",
            Self::AlreadyTried => "this word was already tried",
            Self::MovedCorrectLetter => "hard mode: keep every green letter in its place",
            Self::DroppedFoundLetter => "hard mode: use every letter you have found",
            Self::CannotGiveUp => "only a player still guessing can give up",
            Self::NoGuessPending => "no guess is being made",
            Self::MarksFitNoGuess => "there is no guess these marks fit",
            Self::WordAlreadyChosen => "the word is already chosen",
            Self::FirstPlayerAlreadyChosen => "the first player is already chosen",
            Self::FirstPlayerAgainstSettings => "the settings name another first player",
            Self::WordRevealedEarly => "the word is revealed at the end",
            Self::RevealedWordInvalid => "the revealed word must be a word",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.english())
    }
}
