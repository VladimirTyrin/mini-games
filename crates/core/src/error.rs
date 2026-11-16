//! Error types shared by all games.

use std::fmt;

/// Why a game could not be created.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetupError {
    /// A setting is out of its allowed range.
    InvalidSetting {
        /// Setting name as shown to users.
        field: &'static str,
        /// What is wrong.
        reason: String,
    },
    /// Seat count outside the game's limits.
    PlayerCount {
        /// Requested seats.
        got: usize,
        /// Minimum allowed.
        min: u8,
        /// Maximum allowed.
        max: u8,
    },
    /// Bots requested where the game has none, or other seat problems.
    InvalidPlayers(String),
}

impl SetupError {
    /// Shorthand for [`SetupError::InvalidSetting`].
    pub fn setting(field: &'static str, reason: impl Into<String>) -> Self {
        Self::InvalidSetting {
            field,
            reason: reason.into(),
        }
    }

    /// Checks `min <= value <= max` for a named setting.
    pub fn check_range<T: PartialOrd + fmt::Display + Copy>(
        field: &'static str,
        value: T,
        min: T,
        max: T,
    ) -> Result<(), Self> {
        if value < min || value > max {
            Err(Self::setting(
                field,
                format!("must be between {min} and {max}, got {value}"),
            ))
        } else {
            Ok(())
        }
    }
}

impl fmt::Display for SetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSetting { field, reason } => {
                write!(f, "invalid setting `{field}`: {reason}")
            }
            Self::PlayerCount { got, min, max } => {
                write!(f, "this game needs {min}..={max} players, got {got}")
            }
            Self::InvalidPlayers(reason) => write!(f, "invalid players: {reason}"),
        }
    }
}

impl std::error::Error for SetupError {}

/// Why a command or world event was rejected. Rejection never changes state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandError {
    /// The game is over.
    GameOver,
    /// It is not this seat's turn.
    NotYourTurn,
    /// The seat is out of play (dead, dropped) or does not exist.
    NotPlaying,
    /// The move is illegal in the current position.
    Illegal(&'static str),
    /// Input buffer for this seat is full; try again next tick.
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

/// Why a frame could not be applied by a [`crate::Match`]. Any of these means desync.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameError {
    /// Frames must be applied in order without gaps.
    UnexpectedTick {
        /// Tick the match expected next.
        expected: u64,
        /// Tick the frame carried.
        got: u64,
    },
    /// An input the authority accepted was rejected here.
    Rejected {
        /// Frame tick.
        tick: u64,
        /// Index of the input inside the frame.
        index: usize,
        /// Rejection reason.
        error: CommandError,
    },
    /// Local state hash differs from the authority's.
    HashMismatch {
        /// Frame tick.
        tick: u64,
        /// Authority's hash.
        expected: u64,
        /// Local hash.
        actual: u64,
    },
    /// A seat index outside the table.
    BadSlot {
        /// Frame tick.
        tick: u64,
        /// Offending seat.
        slot: u8,
    },
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedTick { expected, got } => {
                write!(f, "expected frame {expected}, got {got}")
            }
            Self::Rejected { tick, index, error } => {
                write!(f, "frame {tick}: input #{index} rejected: {error}")
            }
            Self::HashMismatch {
                tick,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "frame {tick}: state hash {actual:016x} != authority {expected:016x}"
                )
            }
            Self::BadSlot { tick, slot } => write!(f, "frame {tick}: unknown seat {slot}"),
        }
    }
}

impl std::error::Error for FrameError {}
