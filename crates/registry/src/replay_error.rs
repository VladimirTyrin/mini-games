use mg_core::CompactLogError;
use mg_protocol::ReplayFileError;

use crate::create_error::CreateError;

#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error(transparent)]
    File(#[from] ReplayFileError),
    #[error("replay header is invalid: {0}")]
    Header(#[from] CreateError),
    #[error("replay frames are invalid: {0}")]
    Log(#[from] CompactLogError),
}
