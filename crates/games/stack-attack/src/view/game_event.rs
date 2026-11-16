use mg_core::Slot;

use crate::model::{BoxId, Cell, Column, CraneId, Row};

/// What happened during the latest frame, for animations and sounds. Derived from the state
/// transitions, so it is not part of the state hash.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GameEvent {
    CraneSpawned {
        crane_id: CraneId,
        start_x: Column,
        target_x: Column,
    },
    BoxDropped {
        crane_id: CraneId,
        box_id: BoxId,
        x: Column,
    },
    BoxLanded {
        box_id: BoxId,
        cell: Cell,
    },
    BoxPushed {
        box_id: BoxId,
        from_x: Column,
        to_x: Column,
        by: Slot,
    },
    /// A jumping worker's head broke a falling box; `cell` is where the box was when hit.
    BoxBroken {
        box_id: BoxId,
        cell: Cell,
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
        rows: Vec<Row>,
        points: u32,
    },
}
