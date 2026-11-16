use std::collections::VecDeque;

use crate::io::Command;
use crate::model::{Cell, WorkerState, WorkerStatus};
use crate::rules::WORKER_HEIGHT;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Worker {
    pub(crate) feet: Cell,
    pub(crate) state: WorkerState,
    pub(crate) status: WorkerStatus,
    pub(crate) queue: VecDeque<Command>,
}

impl Worker {
    pub(crate) fn new(feet: Cell) -> Self {
        Self {
            feet,
            state: WorkerState::Grounded,
            status: WorkerStatus::Alive,
            queue: VecDeque::new(),
        }
    }

    pub(crate) fn head(&self) -> Cell {
        Cell::new(self.feet.x, self.feet.y + WORKER_HEIGHT - 1)
    }

    pub(crate) fn is_alive(&self) -> bool {
        self.status == WorkerStatus::Alive
    }

    pub(crate) fn occupies(&self, cell: Cell) -> bool {
        cell.x == self.feet.x && (self.feet.y..=self.head().y).contains(&cell.y)
    }

    pub(crate) fn leave_play(&mut self, status: WorkerStatus) {
        self.status = status;
        self.queue.clear();
    }
}
