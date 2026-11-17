use crate::model::{LetterMark, Word};

/// Marks a guess against the answer. Repeated letters count: a letter is `Present` only as
/// many times as the answer still has it after the exact matches, left to right, so guessing
/// "speed" against "abide" marks one `e` (the `Correct` one), not three.
pub(crate) fn mark_guess(guess: &Word, answer: &Word) -> Vec<LetterMark> {
    let pairs = || guess.letters().iter().zip(answer.letters());
    let mut unmatched: Vec<char> = pairs()
        .filter(|(guessed, wanted)| guessed != wanted)
        .map(|(_, wanted)| *wanted)
        .collect();
    pairs()
        .map(|(guessed, wanted)| {
            if guessed == wanted {
                return LetterMark::Correct;
            }
            match unmatched.iter().position(|letter| letter == guessed) {
                Some(found) => {
                    unmatched.swap_remove(found);
                    LetterMark::Present
                }
                None => LetterMark::Absent,
            }
        })
        .collect()
}
