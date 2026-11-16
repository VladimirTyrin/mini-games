/// Raw choice between a fresh random word and the word of the day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PuzzleMode {
    Unlimited,
    Daily,
}
