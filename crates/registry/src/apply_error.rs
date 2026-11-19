use mg_core::{CommandError, CompactLogError, FrameError};
use mg_protocol::{WireError, WireRefusal};

use crate::rejection::Rejection;

#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error(transparent)]
    Wire(#[from] WireError),
    #[error(transparent)]
    Frame(#[from] FrameError<Rejection>),
    #[error(transparent)]
    Log(#[from] CompactLogError),
}

impl<R: WireRefusal> From<FrameError<CommandError<R>>> for ApplyError {
    fn from(error: FrameError<CommandError<R>>) -> Self {
        Self::Frame(error.map_rejection(Rejection::from))
    }
}
