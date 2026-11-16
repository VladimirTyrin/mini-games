use mg_core::Slot;

use crate::box_id::BoxId;
use crate::cell::Cell;
use crate::crane_id::CraneId;

/// What happened during the latest frame, for animations and sounds. Derived from the state
/// transitions, so it is not part of the state hash.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GameEvent {
    CraneSpawned {
        crane_id: CraneId,
        start_x: i32,
        target_x: i32,
    },
    BoxDropped {
        crane_id: CraneId,
        box_id: BoxId,
        x: i32,
    },
    BoxLanded {
        box_id: BoxId,
        cell: Cell,
    },
    BoxPushed {
        box_id: BoxId,
        from_x: i32,
        to_x: i32,
        by: Slot,
    },
    WorkerJumped {
        slot: Slot,
    },
    WorkerLanded {
        slot: Slot,
        feet: Cell,
    },
    WorkerCrushed {
        slot: Slot,
        box_id: BoxId,
    },
    WorkerLeft {
        slot: Slot,
    },
    /// One event per tick however many rows were cleared; `points` is exactly what was added to
    /// the score (A7).
    LinesCleared {
        rows: Vec<i32>,
        points: u32,
    },
}
