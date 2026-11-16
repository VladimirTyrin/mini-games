use tokio::sync::mpsc;

use super::match_command::MatchCommand;
use crate::ids::MatchId;

#[derive(Clone, Debug)]
pub(crate) struct MatchHandle {
    pub match_id: MatchId,
    sender: mpsc::Sender<MatchCommand>,
}

impl MatchHandle {
    pub(crate) fn new(match_id: MatchId, sender: mpsc::Sender<MatchCommand>) -> Self {
        Self { match_id, sender }
    }

    /// Never blocks the hub: if the actor is badly backed up the command is dropped and logged;
    /// the client can recover through catch-up.
    pub(crate) fn send(&self, command: MatchCommand) {
        if let Err(error) = self.sender.try_send(command) {
            tracing::warn!(match_id = %self.match_id, %error, "match command dropped");
        }
    }
}
