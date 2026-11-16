use crate::model::{LetterMark, ScoredGuess, Word};
use crate::rules::HardModeViolation;

/// Hard mode: a guess must keep every letter an earlier guess found, green ones at their
/// position and found letters at least as many times as one guess revealed them.
pub(crate) fn check_hard_mode(
    guess: &Word,
    earlier: &[ScoredGuess],
) -> Result<(), HardModeViolation> {
    for scored in earlier {
        let pairs = scored.word.letters().iter().zip(&scored.marks);
        for ((found, mark), guessed) in pairs.clone().zip(guess.letters()) {
            if *mark == LetterMark::Correct && found != guessed {
                return Err(HardModeViolation::MovedCorrectLetter);
            }
        }
        for (letter, _) in pairs
            .clone()
            .filter(|(_, mark)| **mark != LetterMark::Absent)
        {
            let revealed = pairs
                .clone()
                .filter(|(other, mark)| *other == letter && **mark != LetterMark::Absent)
                .count();
            let used = guess
                .letters()
                .iter()
                .filter(|other| *other == letter)
                .count();
            if used < revealed {
                return Err(HardModeViolation::DroppedFoundLetter);
            }
        }
    }
    Ok(())
}
