use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::extract::ws::Utf8Bytes;
use mg_protocol::proto;
use tokio::sync::{Notify, mpsc};

use crate::codec;
use crate::ids::ConnectionId;
use crate::limits::Limits;

/// Cheap, cloneable sender to one connection. Never blocks: a full queue marks the connection
/// as overloaded and asks its actor to close it, so one slow client cannot stall the hub or a
/// match broadcast.
#[derive(Clone, Debug)]
pub(crate) struct Outbound {
    connection: ConnectionId,
    sender: mpsc::Sender<Utf8Bytes>,
    overloaded: Arc<AtomicBool>,
    close: Arc<Notify>,
}

pub(crate) struct OutboundReceiver {
    pub receiver: mpsc::Receiver<Utf8Bytes>,
    pub close: Arc<Notify>,
    pub overloaded: Arc<AtomicBool>,
}

impl Outbound {
    pub(crate) fn channel(connection: ConnectionId) -> (Self, OutboundReceiver) {
        let (sender, receiver) = mpsc::channel(Limits::OUTBOUND_QUEUE);
        let overloaded = Arc::new(AtomicBool::new(false));
        let close = Arc::new(Notify::new());
        let outbound = Self {
            connection,
            sender,
            overloaded: overloaded.clone(),
            close: close.clone(),
        };
        (
            outbound,
            OutboundReceiver {
                receiver,
                close,
                overloaded,
            },
        )
    }

    pub(crate) fn send(&self, message: &proto::ServerMessage) {
        self.send_encoded(codec::encode(message));
    }

    /// For broadcasts: encode once, send the same bytes to everyone.
    pub(crate) fn send_encoded(&self, text: Utf8Bytes) {
        match self.sender.try_send(text) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                if !self.overloaded.swap(true, Ordering::Relaxed) {
                    tracing::warn!(connection = %self.connection, "outbound queue full, closing connection");
                    self.close.notify_one();
                }
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {}
        }
    }

    /// Closes the connection after already queued messages are flushed.
    pub(crate) fn close(&self) {
        self.close.notify_one();
    }
}
