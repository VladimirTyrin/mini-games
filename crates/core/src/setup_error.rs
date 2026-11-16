use std::fmt;

/// Why [`crate::Game::new`] rejected parsed settings for these players.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetupError {
    PlayerCount { got: usize, min: u8, max: u8 },
    InvalidPlayers(String),
}

impl SetupError {
    pub fn check_player_count(got: usize, min: u8, max: u8) -> Result<(), Self> {
        if got < usize::from(min) || got > usize::from(max) {
            Err(Self::PlayerCount { got, min, max })
        } else {
            Ok(())
        }
    }
}

impl fmt::Display for SetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlayerCount { got, min, max } => {
                write!(f, "this game needs {min}..={max} players, got {got}")
            }
            Self::InvalidPlayers(reason) => write!(f, "invalid players: {reason}"),
        }
    }
}

impl std::error::Error for SetupError {}
