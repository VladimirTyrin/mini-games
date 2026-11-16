use tokio::sync::mpsc;

use super::hub_command::HubCommand;

/// Unbounded on purpose: the hub only does cheap bookkeeping, and every producer is rate-limited
/// (connections) or bounded in number (match actors), so the queue cannot grow without bound,
/// while a bounded queue could deadlock a match actor reporting its end.
#[derive(Clone, Debug)]
pub(crate) struct HubHandle {
    sender: mpsc::UnboundedSender<HubCommand>,
}

impl HubHandle {
    pub(crate) fn new(sender: mpsc::UnboundedSender<HubCommand>) -> Self {
        Self { sender }
    }

    pub(crate) fn send(&self, command: HubCommand) {
        if self.sender.send(command).is_err() {
            tracing::debug!("hub is gone (shutting down)");
        }
    }
}
