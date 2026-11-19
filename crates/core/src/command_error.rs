use std::fmt;

use crate::english_text::EnglishText;
use crate::protocol_misuse::ProtocolMisuse;

/// Rejection of a command or world event. A rejection never changes state.
///
/// `R` is the game's own reason ([`crate::Game::Refusal`]), so a client words it in the player's
/// language; it also names the game's malformed input (a world event that does not fit), which
/// the wire sends as a generic "malformed". No rejection carries text: `Display` words them in
/// English for logs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandError<R> {
    GameOver,
    NotYourTurn,
    /// The seat is out of play (dead, dropped) or is not a human seat.
    NotPlaying,
    /// The seat's input buffer is full; retry next tick.
    Busy,
    Illegal(R),
    /// The core's own malformed input, outside any game's rules.
    Malformed(ProtocolMisuse),
}

/// The reason alone ("not your turn", "you must capture"), as bot clients read it.
impl<R: EnglishText> EnglishText for CommandError<R> {
    fn english(self) -> &'static str {
        match self {
            Self::GameOver => "the game is over",
            Self::NotYourTurn => "not your turn",
            Self::NotPlaying => "you are not playing",
            Self::Busy => "too many inputs this tick",
            Self::Illegal(refusal) => refusal.english(),
            Self::Malformed(misuse) => misuse.english(),
        }
    }
}

impl<R: EnglishText> fmt::Display for CommandError<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Illegal(refusal) => write!(f, "illegal move: {}", refusal.english()),
            Self::Malformed(misuse) => write!(f, "malformed input: {}", misuse.english()),
            other => f.write_str(other.english()),
        }
    }
}

impl<R: EnglishText + fmt::Debug> std::error::Error for CommandError<R> {}
