use mg_protocol::proto;

use crate::ids::SessionId;
use crate::outbound::Outbound;

#[derive(Debug)]
pub(crate) enum MatchCommand {
    Submit {
        session: SessionId,
        request_id: u32,
        command: proto::GameCommand,
    },
    /// A participant's connection (re)appeared; it receives a catch-up.
    Attach {
        session: SessionId,
        outbound: Outbound,
    },
    /// Connection lost but the session may come back: a seated human pauses the match.
    Detach {
        session: SessionId,
    },
    /// Left for good: a seated human is dropped from play.
    Leave {
        session: SessionId,
    },
    /// A new observer joined mid-match.
    AddObserver {
        session: SessionId,
        outbound: Outbound,
    },
    VoteDrop {
        session: SessionId,
        request_id: u32,
        slot: u32,
    },
    CatchUp {
        session: SessionId,
    },
    Stop,
}
