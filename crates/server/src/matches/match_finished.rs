use std::sync::Arc;

use mg_protocol::proto;

use crate::ids::{LobbyId, MatchId};

#[derive(Debug)]
pub(crate) struct MatchFinished {
    pub lobby: LobbyId,
    pub match_id: MatchId,
    pub result: proto::MatchResult,
    /// Encoded `ReplayFile`.
    pub replay: Arc<[u8]>,
}
