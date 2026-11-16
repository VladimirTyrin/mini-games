/// What the player sends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// A guess as typed; the game parses it (see [`crate::Word::parse`]).
    Guess(String),
    /// Stop: alone the game is lost and the word shown; in a group the seat drops out and the
    /// others play on. Allowed at any time, not only on one's turn.
    GiveUp,
}
