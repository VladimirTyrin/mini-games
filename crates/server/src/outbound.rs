use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use axum::extract::ws::Utf8Bytes;
use mg_protocol::proto;
use tokio::sync::{Notify, mpsc};

use crate::codec;
use crate::ids::ConnectionId;
use crate::limits::Limits;

#[derive(Debug, Default)]
struct OutboundShared {
    overloaded: AtomicBool,
    queued_bytes: AtomicUsize,
    close: Notify,
}

/// Cheap, cloneable sender to one connection. Never blocks: a queue over its count or byte cap
/// marks the connection as overloaded and asks its writer to close it, so one slow client cannot
/// stall the hub or a match broadcast. Nothing is queued after that, so the client never sees a
/// stream with silent gaps; it reconnects and resumes instead.
#[derive(Clone, Debug)]
pub(crate) struct Outbound {
    connection: ConnectionId,
    sender: mpsc::Sender<Utf8Bytes>,
    shared: Arc<OutboundShared>,
}

pub(crate) struct OutboundReceiver {
    receiver: mpsc::Receiver<Utf8Bytes>,
    shared: Arc<OutboundShared>,
}

impl Outbound {
    pub(crate) fn channel(connection: ConnectionId) -> (Self, OutboundReceiver) {
        let (sender, receiver) = mpsc::channel(Limits::OUTBOUND_QUEUE);
        let shared = Arc::new(OutboundShared::default());
        let outbound = Self {
            connection,
            sender,
            shared: shared.clone(),
        };
        (outbound, OutboundReceiver { receiver, shared })
    }

    pub(crate) fn send(&self, message: &proto::ServerMessage) {
        self.send_encoded(codec::encode(message));
    }

    /// For broadcasts: encode once, send the same bytes to everyone.
    pub(crate) fn send_encoded(&self, text: Utf8Bytes) {
        if self.shared.overloaded.load(Ordering::Relaxed) {
            return;
        }
        let bytes = text.len();
        let queued_before = self.shared.queued_bytes.fetch_add(bytes, Ordering::Relaxed);
        if queued_before > 0 && queued_before.saturating_add(bytes) > Limits::OUTBOUND_BYTES {
            self.shared.queued_bytes.fetch_sub(bytes, Ordering::Relaxed);
            return self.overload();
        }
        match self.sender.try_send(text) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.shared.queued_bytes.fetch_sub(bytes, Ordering::Relaxed);
                self.overload();
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.shared.queued_bytes.fetch_sub(bytes, Ordering::Relaxed);
            }
        }
    }

    fn overload(&self) {
        if !self.shared.overloaded.swap(true, Ordering::Relaxed) {
            tracing::warn!(connection = %self.connection, "outbound queue full, closing connection");
            self.shared.close.notify_one();
        }
    }

    /// Closes the connection after already queued messages are flushed.
    pub(crate) fn close(&self) {
        self.shared.close.notify_one();
    }
}

impl OutboundReceiver {
    /// The next queued message; `None` once every sender is gone.
    pub(crate) async fn next(&mut self) -> Option<Utf8Bytes> {
        let text = self.receiver.recv().await?;
        self.shared
            .queued_bytes
            .fetch_sub(text.len(), Ordering::Relaxed);
        Some(text)
    }

    pub(crate) fn try_next(&mut self) -> Option<Utf8Bytes> {
        let text = self.receiver.try_recv().ok()?;
        self.shared
            .queued_bytes
            .fetch_sub(text.len(), Ordering::Relaxed);
        Some(text)
    }

    /// Resolves when a close was requested. Owned, so a writer can wait on it while it also
    /// takes messages from the receiver.
    pub(crate) fn close_requested(&self) -> impl Future<Output = ()> + use<> {
        let shared = self.shared.clone();
        async move { shared.close.notified().await }
    }

    /// An overloaded client gets nothing more, not even what is still queued: flushing would
    /// block on the very client that could not keep up.
    pub(crate) fn is_overloaded(&self) -> bool {
        self.shared.overloaded.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(bytes: usize) -> Utf8Bytes {
        "x".repeat(bytes).into()
    }

    #[test]
    fn byte_cap_overloads_and_nothing_is_queued_afterwards() {
        let (outbound, mut receiver) = Outbound::channel(ConnectionId::next());
        // One message above the cap is fine on an empty queue (a replay download).
        outbound.send_encoded(message(Limits::OUTBOUND_BYTES + 1));
        assert!(!receiver.is_overloaded());
        outbound.send_encoded(message(10));
        assert!(receiver.is_overloaded());
        outbound.send_encoded(message(10));
        assert_eq!(
            receiver.try_next().map(|text| text.len()),
            Some(Limits::OUTBOUND_BYTES + 1)
        );
        assert!(receiver.try_next().is_none());
    }

    #[test]
    fn count_cap_overloads_and_stops_queueing() {
        let (outbound, mut receiver) = Outbound::channel(ConnectionId::next());
        for _ in 0..Limits::OUTBOUND_QUEUE + 5 {
            outbound.send_encoded(message(1));
        }
        assert!(receiver.is_overloaded());
        let mut queued = 0;
        while receiver.try_next().is_some() {
            queued += 1;
        }
        assert_eq!(queued, Limits::OUTBOUND_QUEUE);
        outbound.send_encoded(message(1));
        assert!(receiver.try_next().is_none());
    }
}
