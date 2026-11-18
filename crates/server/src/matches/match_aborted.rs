use crate::ids::{LobbyId, MatchId};

/// A match actor ended without a result (panic, lost authority, or every player away).
#[derive(Debug)]
pub(crate) struct MatchAborted {
    pub lobby: LobbyId,
    pub match_id: MatchId,
}
