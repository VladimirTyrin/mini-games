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

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GameOver => f.write_str("the game is over"),
            Self::NotYourTurn => f.write_str("not your turn"),
            Self::NotPlaying => f.write_str("you are not playing"),
            Self::Illegal(why) => write!(f, "illegal move: {why}"),
            Self::Busy => f.write_str("too many inputs this tick"),
        }
    }
}

impl std::error::Error for CommandError {}
