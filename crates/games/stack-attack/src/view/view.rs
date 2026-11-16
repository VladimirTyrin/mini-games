use mg_core::Slot;

use crate::model::{CargoBox, Crane, Level, Score, TickCount};
use crate::rules::EndReason;
use crate::view::{GameEvent, WorkerView};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub you: Option<Slot>,
    pub field_width: u32,
    pub field_height: u32,
    pub tick_ms: u32,
    /// Indexed by seat, including crushed and departed workers.
    pub workers: Vec<WorkerView>,
    /// Sorted by id.
    pub boxes: Vec<CargoBox>,
    pub cranes: Vec<Crane>,
    pub score: Score,
    pub lines_cleared: u32,
    pub boxes_pushed: u32,
    pub level: Level,
    /// Survival time (A10); multiply by `tick_ms` for wall time.
    pub ticks_survived: TickCount,
    pub events: Vec<GameEvent>,
    pub end_reason: Option<EndReason>,
}
