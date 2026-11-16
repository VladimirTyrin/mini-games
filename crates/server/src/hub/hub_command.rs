use mg_protocol::proto;
use tokio::sync::oneshot;

use crate::authenticated::Authenticated;
use crate::ids::{ConnectionId, ResumeToken, SessionId};
use crate::matches::MatchFinished;
use crate::outbound::Outbound;

#[derive(Debug)]
pub(crate) struct SignIn {
    pub connection: ConnectionId,
    pub outbound: Outbound,
    pub who: Authenticated,
    /// Empty for guests.
    pub session_token: String,
    pub resume: Option<ResumeToken>,
    pub request_id: u32,
    /// `None` when sign-in was refused (the hub already sent the error).
    pub reply: oneshot::Sender<Option<SessionId>>,
}

#[derive(Debug)]
pub(crate) enum HubCommand {
    SignIn(SignIn),
    Client {
        session: SessionId,
        connection: ConnectionId,
        request_id: u32,
        message: proto::client_message::Message,
    },
    Disconnected {
        session: SessionId,
        connection: ConnectionId,
    },
    SignOut {
        session: SessionId,
        connection: ConnectionId,
    },
    MatchFinished(MatchFinished),
    ResumeGraceExpired {
        session: SessionId,
        connection: ConnectionId,
    },
    Housekeeping,
    Shutdown,
}
