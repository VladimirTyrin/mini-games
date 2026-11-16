use mg_protocol::proto;

use crate::authenticated::Authenticated;
use crate::ids::{ConnectionId, LobbyId, ResumeToken};
use crate::outbound::Outbound;

#[derive(Debug)]
pub(super) struct Session {
    pub who: Authenticated,
    pub identity: proto::Identity,
    pub resume_token: ResumeToken,
    /// `None` while disconnected (within the resume grace period).
    pub connection: Option<SessionConnection>,
    /// The connection whose loss started the current resume grace period.
    pub lost_connection: Option<ConnectionId>,
    pub lobby: Option<LobbyId>,
}

#[derive(Debug)]
pub(super) struct SessionConnection {
    pub id: ConnectionId,
    pub outbound: Outbound,
}

impl Session {
    pub(super) fn outbound(&self) -> Option<&Outbound> {
        self.connection.as_ref().map(|c| &c.outbound)
    }

    pub(super) fn send(&self, message: &proto::ServerMessage) {
        if let Some(outbound) = self.outbound() {
            outbound.send(message);
        }
    }

    pub(super) fn is_connection(&self, connection: ConnectionId) -> bool {
        self.connection.as_ref().is_some_and(|c| c.id == connection)
    }
}
