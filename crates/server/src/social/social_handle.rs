use tokio::sync::mpsc;

use super::SocialCommand;

/// Unbounded like the hub's queue: the hub must never wait on the social actor (which waits on
/// the hub for presence), and its producers are rate-limited connections and the hub itself.
#[derive(Clone, Debug)]
pub(crate) struct SocialHandle {
    sender: mpsc::UnboundedSender<SocialCommand>,
}

impl SocialHandle {
    pub(crate) fn new(sender: mpsc::UnboundedSender<SocialCommand>) -> Self {
        Self { sender }
    }

    pub(crate) fn send(&self, command: SocialCommand) {
        if self.sender.send(command).is_err() {
            tracing::debug!("social actor is gone (shutting down)");
        }
    }
}
