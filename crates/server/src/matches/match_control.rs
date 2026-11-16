use crate::ids::SessionId;
use crate::outbound::Outbound;

/// Lifecycle commands: never dropped, handled before any request.
#[derive(Debug)]
pub(crate) enum MatchControl {
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
    Stop,
}
