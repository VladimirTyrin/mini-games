use crate::model::WordError;
use crate::settings::{Language, WordLength};

/// A word of the game's alphabet and length, letters normalized (see [`Language::normalize`]).
/// Whether it is a real word is the client's check against the word list: the rules only need
/// the letters, so a replay never depends on a word list version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    letters: Vec<char>,
}

impl Word {
    pub fn parse(text: &str, language: Language, length: WordLength) -> Result<Self, WordError> {
        let expected = usize::from(length.to_u8());
        // Bounded before anything is allocated from the input.
        if text.chars().nth(expected).is_some() {
            return Err(WordError::WrongLength);
        }
        let letters = text
            .chars()
            .map(|letter| language.normalize(letter).ok_or(WordError::NotALetter))
            .collect::<Result<Vec<char>, WordError>>()?;
        if letters.len() != expected {
            return Err(WordError::WrongLength);
        }
        Ok(Self { letters })
    }

    pub fn letters(&self) -> &[char] {
        &self.letters
    }

    pub fn text(&self) -> String {
        self.letters.iter().collect()
    }
}
