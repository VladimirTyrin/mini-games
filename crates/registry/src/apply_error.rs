use mg_core::{CompactLogError, FrameError};
use mg_protocol::WireError;

#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error(transparent)]
    Wire(#[from] WireError),
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error(transparent)]
    Log(#[from] CompactLogError),
}
