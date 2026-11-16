use mg_core::SetupError;
use mg_protocol::WireError;

use crate::seats_error::SeatsError;

#[derive(Debug, thiserror::Error)]
pub enum CreateError {
    #[error(transparent)]
    Wire(#[from] WireError),
    #[error(transparent)]
    Seats(#[from] SeatsError),
    #[error(transparent)]
    Setup(#[from] SetupError),
    #[error("{0} has no built-in bots")]
    NoBots(&'static str),
}
