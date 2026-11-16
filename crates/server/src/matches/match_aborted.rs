use crate::ids::{LobbyId, MatchId};

/// A match actor died without a result (panic or lost authority).
#[derive(Debug)]
pub(crate) struct MatchAborted {
    pub lobby: LobbyId,
    pub match_id: MatchId,
}
