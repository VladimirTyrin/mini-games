use std::fmt;

use mg_protocol::proto;

/// An `ErrorResponse` from the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerError {
    pub code: proto::ErrorCode,
    /// Human-readable; never parse it.
    pub message: String,
}

impl ServerError {
    pub(crate) fn from_wire(error: proto::ErrorResponse) -> Self {
        Self {
            code: proto::ErrorCode::try_from(error.code).unwrap_or(proto::ErrorCode::Unspecified),
            message: error.message,
        }
    }
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str_name(), self.message)
    }
}

impl std::error::Error for ServerError {}
