use crate::model::LetterMark;

/// The best mark a letter has got so far, for the on-screen keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyHint {
    pub letter: char,
    pub mark: LetterMark,
}
