use std::fmt;

/// Rejection of a command or world event. A rejection never changes state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandError {
    GameOver,
    NotYourTurn,
    /// The seat is out of play (dead, dropped) or is not a human seat.
    NotPlaying,
    Illegal(&'static str),
    /// The seat's input buffer is full; retry next tick.
    Busy,
}

impl CommandError {
    /// Why, in words a player reads (an illegal move's own reason, without the "illegal move"
    /// prefix [`fmt::Display`] adds for logs).
    pub fn explanation(&self) -> &'static str {
        match self {
            Self::GameOver => "the game is over",
            Self::NotYourTurn => "not your turn",
            Self::NotPlaying => "you are not playing",
            Self::Illegal(why) => why,
            Self::Busy => "too many inputs this tick",
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Illegal(why) => write!(f, "illegal move: {why}"),
            other => f.write_str(other.explanation()),
        }
    }
}

impl std::error::Error for CommandError {}
