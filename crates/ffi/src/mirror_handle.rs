use mg_core::CompactLog;
use mg_protocol::proto;
use mg_registry::DynMirror;

/// Receiving side of lockstep (frames from a server or a replay file). Opaque to C.
#[derive(Debug)]
pub struct MgMirror {
    pub(crate) mirror: Box<dyn DynMirror>,
    /// Set for mirrors made from a replay file; stepped through by `mg_mirror_advance_to`.
    pub(crate) replay_log: Option<CompactLog<proto::TickFrame>>,
}
