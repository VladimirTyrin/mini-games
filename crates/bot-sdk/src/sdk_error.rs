use mg_registry::{ApplyError, CreateError};
use tokio_tungstenite::tungstenite;

use crate::server_error::ServerError;

#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("cannot connect: {0}")]
    Connect(#[source] tungstenite::Error),
    #[error("connection lost")]
    ConnectionClosed,
    #[error("websocket error: {0}")]
    WebSocket(#[source] tungstenite::Error),
    #[error("cannot encode a client message: {0}")]
    Encode(#[source] serde_json::Error),
    #[error("cannot decode a server message: {0}")]
    Decode(#[source] serde_json::Error),
    #[error("server refused: {0}")]
    Server(#[source] ServerError),
    #[error("expected {expected}, the server sent {got}")]
    UnexpectedReply { expected: &'static str, got: String },
    #[error("the account signed in on another connection")]
    SignedInElsewhere,
    #[error("the server is shutting down")]
    ServerShuttingDown,
    #[error("the lobby was closed")]
    LobbyClosed,
    #[error("the server did not restore the session after reconnecting; the seat is lost")]
    SeatLost,
    #[error("gave up reconnecting after {attempts} attempts: {last}")]
    ReconnectFailed {
        attempts: u32,
        #[source]
        last: Box<SdkError>,
    },
    #[error("cannot mirror the match: {0}")]
    Mirror(#[source] CreateError),
    #[error("the server's catch-up does not apply: {0}")]
    CatchUp(#[source] ApplyError),
}

impl SdkError {
    /// Failures a new connection may cure.
    pub(crate) fn is_connection_loss(&self) -> bool {
        matches!(
            self,
            Self::Connect(_) | Self::ConnectionClosed | Self::WebSocket(_)
        )
    }
}
