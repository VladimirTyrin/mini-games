use std::sync::Arc;

use mg_protocol::proto;

use crate::ids::{LobbyId, MatchId};

#[derive(Debug)]
pub(crate) struct MatchFinished {
    pub lobby: LobbyId,
    pub match_id: MatchId,
    pub result: proto::MatchResult,
    /// `ReplayData` already in JSON, encoded once by the actor and spliced into every reply.
    pub replay_data_json: Arc<str>,
}
