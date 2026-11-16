/// What a guessed letter tells about the answer. Ordered by how much it reveals, so the best
/// mark of a letter over several guesses is the maximum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LetterMark {
    /// Not in the answer (or not that many times).
    Absent,
    /// In the answer, somewhere else.
    Present,
    /// In the answer at this position.
    Correct,
}
