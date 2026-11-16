use mg_protocol::proto;

use crate::ids::SessionId;

#[derive(Debug, Clone)]
pub(super) enum LobbySeat {
    Human {
        session: SessionId,
        ready: bool,
    },
    Bot {
        kind: proto::BotKind,
        identity: proto::Identity,
    },
}

impl LobbySeat {
    pub(super) fn session(&self) -> Option<&SessionId> {
        match self {
            Self::Human { session, .. } => Some(session),
            Self::Bot { .. } => None,
        }
    }
}
