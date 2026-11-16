/// Randomness is resolved by the authority and sent to everyone as a world event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldEvent {
    /// The answer. Carrying the word itself (not an index into the list) keeps replays valid
    /// when the answer list changes.
    AnswerChosen(String),
}
