use mg_core::CommandError;
use mg_protocol::{WireError, WireRefusal};

use crate::rejection::Rejection;

#[derive(Debug, thiserror::Error)]
pub enum SubmitError {
    #[error(transparent)]
    Wire(#[from] WireError),
    #[error(transparent)]
    Rejected(#[from] Rejection),
}

impl<R: WireRefusal> From<CommandError<R>> for SubmitError {
    fn from(error: CommandError<R>) -> Self {
        Self::Rejected(error.into())
    }
}
