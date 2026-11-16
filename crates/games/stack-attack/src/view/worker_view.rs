use mg_core::Slot;

use crate::model::{Cell, WorkerState, WorkerStatus};

/// The UI derives the worker colour from `slot`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WorkerView {
    pub slot: Slot,
    /// The worker also occupies the cell above.
    pub feet: Cell,
    pub state: WorkerState,
    pub status: WorkerStatus,
    pub queued_commands: usize,
}
