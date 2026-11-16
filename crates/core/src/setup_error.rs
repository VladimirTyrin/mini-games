use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetupError {
    InvalidSetting { field: &'static str, reason: String },
    PlayerCount { got: usize, min: u8, max: u8 },
    InvalidPlayers(String),
}

impl SetupError {
    pub fn setting(field: &'static str, reason: impl Into<String>) -> Self {
        Self::InvalidSetting {
            field,
            reason: reason.into(),
        }
    }

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
