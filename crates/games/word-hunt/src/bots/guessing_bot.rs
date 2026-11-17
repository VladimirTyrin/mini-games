use mg_core::{Awaiting, Bot, Budget, CoreRng, Game, Slot};

use super::BotKind;
use crate::game::WordHunt;
use crate::io::Command;
use crate::model::{LetterMark, ScoredGuess, Word};
use crate::rules::{check_hard_mode, mark_guess};

/// Guesses a word from the answer list that fits the hints its level takes into account. It
/// reads only the guesses on the board, never the answer.
#[derive(Clone, Copy, Debug)]
pub struct GuessingBot {
    kind: BotKind,
}

impl GuessingBot {
    pub fn new(kind: BotKind) -> Self {
        Self { kind }
    }

    /// Would `candidate`, were it the answer, have produced exactly these marks?
    fn explains(candidate: &Word, guess: &ScoredGuess) -> bool {
        mark_guess(&guess.word, candidate) == guess.marks
    }

    /// The green and yellow marks hold for `candidate`; greys are not checked.
    fn keeps_found_letters(candidate: &Word, guess: &ScoredGuess) -> bool {
        let at = |position: usize| candidate.letters().get(position);
        guess
            .word
            .letters()
            .iter()
            .zip(&guess.marks)
            .enumerate()
            .all(|(position, (letter, mark))| match mark {
                LetterMark::Correct => at(position) == Some(letter),
                LetterMark::Present => {
                    at(position) != Some(letter) && candidate.letters().contains(letter)
                }
                LetterMark::Absent => true,
            })
    }

    fn fits(self, candidate: &Word, guesses: &[ScoredGuess]) -> bool {
        match self.kind {
            BotKind::Easy => guesses
                .iter()
                .rev()
                .take(2)
                .all(|guess| Self::explains(candidate, guess)),
            BotKind::Medium => guesses
                .iter()
                .all(|guess| Self::keeps_found_letters(candidate, guess)),
            BotKind::Hard => guesses.iter().all(|guess| Self::explains(candidate, guess)),
        }
    }
}

impl Bot<WordHunt> for GuessingBot {
    fn decide(
        &mut self,
        game: &WordHunt,
        me: Slot,
        rng: &mut CoreRng,
        budget: &mut Budget,
    ) -> Option<Command> {
        if game.awaiting() != Awaiting::Turn(me) {
            return None;
        }
        let settings = game.settings();
        let guesses = game.guesses();
        let legal = |word: &Word| {
            !guesses.iter().any(|guess| guess.word == *word)
                && (!settings.hard_mode || check_hard_mode(word, guesses).is_ok())
        };
        let words: Vec<Word> = game
            .answers()
            .all()
            .filter_map(|text| Word::parse(text, settings.language, settings.word_length()).ok())
            .collect();
        let mut fitting = Vec::new();
        for word in &words {
            if !budget.spend(u64::try_from(guesses.len()).unwrap_or(u64::MAX).max(1)) {
                break;
            }
            if legal(word) && self.fits(word, guesses) {
                fitting.push(word);
            }
        }
        // The answer always fits every hint, so this only runs out on a spent budget; any legal
        // word still keeps the game going.
        let chosen = match rng.pick(&fitting) {
            Some(word) => (*word).clone(),
            None => {
                let legal_words: Vec<&Word> = words.iter().filter(|word| legal(word)).collect();
                (*rng.pick(&legal_words)?).clone()
            }
        };
        Some(Command::Guess(chosen.text()))
    }
}
