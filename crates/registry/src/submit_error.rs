use mg_core::CommandError;
use mg_protocol::WireError;

#[derive(Debug, thiserror::Error)]
pub enum SubmitError {
    #[error(transparent)]
    Wire(#[from] WireError),
    #[error(transparent)]
    Rejected(#[from] CommandError),
}
