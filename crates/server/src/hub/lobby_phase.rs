use std::collections::BTreeSet;

use mg_protocol::proto;

use super::shared_replay::SharedReplay;
use crate::ids::SessionId;
use crate::matches::MatchHandle;

#[derive(Debug)]
pub(super) enum LobbyPhase {
    Open,
    InMatch(MatchHandle),
    PostMatch {
        play_again_votes: BTreeSet<SessionId>,
    },
    Replay(SharedReplay),
}

impl LobbyPhase {
    pub(super) fn to_wire(&self) -> proto::LobbyPhase {
        match self {
            Self::Open => proto::LobbyPhase::Open,
            Self::InMatch(_) => proto::LobbyPhase::InMatch,
            Self::PostMatch { .. } => proto::LobbyPhase::PostMatch,
            Self::Replay(_) => proto::LobbyPhase::Replay,
        }
    }

    /// Seats, settings and bots may change only between matches.
    pub(super) fn allows_setup_changes(&self) -> bool {
        matches!(self, Self::Open | Self::PostMatch { .. })
    }

    pub(super) fn match_handle(&self) -> Option<&MatchHandle> {
        match self {
            Self::InMatch(handle) => Some(handle),
            _ => None,
        }
    }
}
