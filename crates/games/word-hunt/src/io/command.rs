/// What the player sends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// A guess as typed; the game parses it (see [`crate::Word::parse`]).
    Guess(String),
}
