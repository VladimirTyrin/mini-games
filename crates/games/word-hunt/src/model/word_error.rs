/// Why a text is not a word of this game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordError {
    WrongLength,
    NotALetter,
}

impl WordError {
    pub(crate) const fn reason(self) -> &'static str {
        match self {
            WordError::WrongLength => "the word has the wrong number of letters",
            WordError::NotALetter => "the word has a character outside the alphabet",
        }
    }
}
