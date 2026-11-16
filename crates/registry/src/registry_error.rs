use mg_core::{CommandError, FrameError, SetupError};
use mg_protocol::WireError;

#[derive(Debug, thiserror::Error)]
pub enum CreateError {
    #[error(transparent)]
    Wire(#[from] WireError),
    #[error(transparent)]
    Setup(#[from] SetupError),
    #[error("{0} has no built-in bots")]
    NoBots(&'static str),
}

#[derive(Debug, thiserror::Error)]
pub enum SubmitError {
    #[error(transparent)]
    Wire(#[from] WireError),
    #[error(transparent)]
    Rejected(#[from] CommandError),
}

#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error(transparent)]
    Wire(#[from] WireError),
    #[error(transparent)]
    Frame(#[from] FrameError),
}
