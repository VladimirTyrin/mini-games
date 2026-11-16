use crate::model::{LetterMark, Word};

/// A guess and the mark of each of its letters, position by position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoredGuess {
    pub word: Word,
    pub marks: Vec<LetterMark>,
}

impl ScoredGuess {
    pub fn is_solved(&self) -> bool {
        self.marks.iter().all(|mark| *mark == LetterMark::Correct)
    }
}
