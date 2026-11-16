use mg_protocol::proto;

use super::request_cooldowns::RequestCooldowns;
use crate::auth::DisplayName;
use crate::authenticated::Authenticated;
use crate::ids::{ConnectionId, LobbyId, ResumeToken};
use crate::outbound::Outbound;

#[derive(Debug)]
pub(super) struct Session {
    pub who: Authenticated,
    /// The name shown online: `who`'s name, with a suffix for an account whose name was taken.
    pub display_name: DisplayName,
    pub identity: proto::Identity,
    pub resume_token: ResumeToken,
    /// `None` while disconnected (within the resume grace period).
    pub connection: Option<SessionConnection>,
    /// The connection whose loss started the current resume grace period.
    pub lost_connection: Option<ConnectionId>,
    pub lobby: Option<LobbyId>,
    pub cooldowns: RequestCooldowns,
    /// An uploaded replay is being validated; one at a time per session.
    pub validating_replay: bool,
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
