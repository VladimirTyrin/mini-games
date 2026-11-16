use mg_protocol::proto;

use crate::ids::MatchId;

/// The lobby's last finished match, shown in `POST_MATCH`.
#[derive(Debug)]
pub(super) struct FinishedMatch {
    pub match_id: MatchId,
    pub result: proto::MatchResult,
}
