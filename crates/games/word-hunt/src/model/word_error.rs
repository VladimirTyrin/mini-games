use crate::rules::Refusal;

/// Why a text is not a word of this game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordError {
    WrongLength,
    NotALetter,
}

impl From<WordError> for Refusal {
    fn from(error: WordError) -> Self {
        match error {
            WordError::WrongLength => Refusal::WrongLength,
            WordError::NotALetter => Refusal::NotALetter,
        }
    }
}
