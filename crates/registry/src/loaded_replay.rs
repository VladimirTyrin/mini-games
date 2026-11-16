use mg_core::CompactLog;
use mg_protocol::proto;

use crate::dyn_mirror::DynMirror;

/// A replay file that passed every check short of replaying it.
#[derive(Debug)]
pub struct LoadedReplay {
    /// At tick 0; step it through `log` with [`DynMirror::apply_compact_log`].
    pub mirror: Box<dyn DynMirror>,
    pub log: CompactLog<proto::TickFrame>,
}
