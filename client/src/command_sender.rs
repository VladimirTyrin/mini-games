use tokio::sync::mpsc;
use crate::state::ClientCommand;

#[derive(Clone)]
pub enum CommandSender {
    Grpc(mpsc::UnboundedSender<ClientCommand>),
    Local(mpsc::UnboundedSender<ClientCommand>),
}

impl CommandSender {
    pub fn send(&self, cmd: ClientCommand) {
        let tx = match self {
            Self::Grpc(tx) => tx,
            Self::Local(tx) => tx,
        };
        let _ = tx.send(cmd);
    }
}
