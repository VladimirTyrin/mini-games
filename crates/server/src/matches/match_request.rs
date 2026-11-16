use mg_protocol::proto;

use crate::ids::SessionId;

/// Client requests: queued with a bound and shed (the client gets `RATE_LIMITED`) when the
/// actor falls behind.
#[derive(Debug)]
pub(crate) enum MatchRequest {
    Submit {
        session: SessionId,
        request_id: u32,
        command: proto::GameCommand,
    },
    VoteDrop {
        session: SessionId,
        request_id: u32,
        slot: u32,
    },
    CatchUp {
        session: SessionId,
    },
}
