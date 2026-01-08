use tokio::sync::mpsc;
use common::log;
use crate::state::ClientCommand;

#[derive(Clone)]
pub enum CommandSender {
    Grpc(mpsc::UnboundedSender<ClientCommand>),
    Local(mpsc::UnboundedSender<ClientCommand>),
}

impl CommandSender {
    pub fn send(&self, cmd: ClientCommand) {
        let (tx, channel_type) = match self {
            Self::Grpc(tx) => (tx, "grpc"),
            Self::Local(tx) => (tx, "local"),
        };
        if let Err(e) = tx.send(cmd) {
            log!("Failed to send command to {} channel: {}", channel_type, e);
        }
    }
}
