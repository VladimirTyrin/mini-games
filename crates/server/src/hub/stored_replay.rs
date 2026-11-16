use std::sync::Arc;

use crate::ids::MatchId;

#[derive(Debug)]
pub(super) struct StoredReplay {
    pub match_id: MatchId,
    /// `ReplayData` in JSON; replies splice it in instead of re-encoding megabytes of base64.
    pub replay_data_json: Arc<str>,
}
