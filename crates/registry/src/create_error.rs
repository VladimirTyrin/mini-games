use mg_core::{SettingsError, SetupError};
use mg_protocol::{WireError, proto, setting_problem_to_wire};

use crate::seats_error::SeatsError;

#[derive(Debug, thiserror::Error)]
pub enum CreateError {
    #[error(transparent)]
    Wire(#[from] WireError),
    #[error(transparent)]
    Seats(#[from] SeatsError),
    #[error(transparent)]
    Settings(#[from] SettingsError),
    #[error(transparent)]
    Setup(#[from] SetupError),
    #[error("{0} has no built-in bots")]
    NoBots(&'static str),
}

impl CreateError {
    /// The one setting at fault, for the wire; `None` when the problem is not a single setting
    /// (the wrong game, the seats).
    pub fn setting_problem(&self) -> Option<proto::SettingProblem> {
        match self {
            Self::Settings(error) => Some(setting_problem_to_wire(error)),
            _ => None,
        }
    }
}
