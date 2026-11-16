use mg_core::Slot;
use mg_protocol::proto;
use mg_registry::{CreateError, DynMirror};

/// Decides a bot's commands. The SDK owns the mirror and keeps it in lockstep with the server;
/// the strategy only builds it and reads it.
pub trait Strategy {
    type Mirror: DynMirror;

    /// Called when a match starts and whenever the SDK rebuilds the mirror from a catch-up.
    fn create_mirror(&mut self, header: &proto::MatchHeader) -> Result<Self::Mirror, CreateError>;

    /// Called once per new state while the match runs: at the start, after every frame and after
    /// a catch-up, whoever is to move. Return the command to submit, or `None` to pass (not your
    /// turn, nothing to change). A refused command is counted in the summary, not retried.
    fn decide(&mut self, mirror: &Self::Mirror, me: Slot) -> Option<proto::GameCommand>;
}
