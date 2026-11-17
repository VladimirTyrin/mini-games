use std::collections::BTreeMap;

use mg_protocol::proto;
use tokio::sync::oneshot;

use super::validated_replay::ValidatedReplay;
use crate::authenticated::Authenticated;
use crate::ids::{ConnectionId, ResumeToken, SessionId};
use crate::matches::{MatchAborted, MatchFinished};
use crate::outbound::Outbound;
use crate::social::AccountId;

#[derive(Debug)]
pub(crate) struct SignIn {
    pub connection: ConnectionId,
    pub outbound: Outbound,
    pub who: Authenticated,
    /// Empty for guests.
    pub session_token: String,
    pub resume: Option<ResumeToken>,
    pub server_info: proto::ServerInfo,
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
    MatchAborted(MatchAborted),
    ReplayValidated(ValidatedReplay),
    ResumeGraceExpired {
        session: SessionId,
        connection: ConnectionId,
    },
    /// From the social actor: send to every connected device of the account.
    DeliverToAccount {
        account: AccountId,
        message: proto::ServerMessage,
    },
    DeliverToSession {
        session: SessionId,
        message: proto::ServerMessage,
    },
    PresenceOf {
        accounts: Vec<AccountId>,
        reply: oneshot::Sender<BTreeMap<AccountId, proto::Presence>>,
    },
    /// deploy.sh is about to restart the server: stop starting matches. Answers how many are
    /// still running.
    Drain {
        reply: oneshot::Sender<usize>,
    },
    FlushLobbyList,
    Housekeeping,
    Shutdown,
}
