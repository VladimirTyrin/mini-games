use mg_core::Slot;

use crate::cargo_box::CargoBox;
use crate::cell::Cell;
use crate::command::Command;
use crate::crane_state::CraneState;
use crate::direction::Direction;
use crate::end_reason::EndReason;
use crate::game_event::GameEvent;
use crate::rules::line_clear_points;
use crate::stack_attack::StackAttack;
use crate::worker_state::WorkerState;
use crate::worker_status::WorkerStatus;

impl StackAttack {
    /// Phase order matters:
    /// * worker actions run before gravity, so a jump-then-move lands on the box it reached;
    /// * crush detection runs after line clears, which can shift boxes down into a worker;
    /// * the ceiling check runs last, after clears (A6).
    pub(crate) fn run_tick(&mut self) {
        let mut events = std::mem::take(&mut self.pending_events);
        self.run_worker_actions(&mut events);
        self.update_cranes(&mut events);
        for (box_id, cell) in self.field.apply_gravity() {
            events.push(GameEvent::BoxLanded { box_id, cell });
        }
        self.apply_worker_gravity(&mut events);
        self.clear_lines(&mut events);
        self.crush_workers(&mut events);
        if self.field.has_resting_box_in_top_row() {
            self.end_reason = Some(EndReason::CeilingReached);
        } else if !self.any_worker_alive() {
            self.end_reason = Some(EndReason::WorkersCrushed);
        }
        self.ticks_survived = self.ticks_survived.saturating_add(1);
        self.ticks_since_last_crane = self.ticks_since_last_crane.saturating_add(1);
        self.last_frame_events = events;
    }

    /// One queued action per worker, in seat order (A3).
    fn run_worker_actions(&mut self, events: &mut Vec<GameEvent>) {
        for (seat, slot) in Slot::all(self.workers.len()).enumerate() {
            let Some(worker) = self.workers.get_mut(seat).filter(|w| w.is_alive()) else {
                continue;
            };
            match worker.queue.pop_front() {
                Some(Command::Move(direction)) => self.move_worker(seat, slot, direction, events),
                Some(Command::Jump) => self.jump_worker(seat, slot, events),
                None => {}
            }
        }
    }

    fn living_worker_at(&self, cell: Cell) -> bool {
        self.workers
            .iter()
            .any(|w| w.is_alive() && w.occupies(cell))
    }

    fn move_worker(
        &mut self,
        seat: usize,
        slot: Slot,
        direction: Direction,
        events: &mut Vec<GameEvent>,
    ) {
        let Some(worker) = self.workers.get(seat) else {
            return;
        };
        let target = worker.feet.shifted_x(direction.dx());
        let target_head = worker.head().shifted_x(direction.dx());
        if !self.field.contains(target) || self.field.is_occupied(target_head) {
            return;
        }
        if let Some(box_id) = self.field.box_at(target) {
            let push_to = target.shifted_x(direction.dx());
            // Deliberate change: legacy let a box be pushed into another worker, crushing them.
            if self.living_worker_at(push_to) || !self.field.try_move(box_id, push_to) {
                return;
            }
            self.boxes_pushed = self.boxes_pushed.saturating_add(1);
            events.push(GameEvent::BoxPushed {
                box_id,
                from_x: target.x,
                to_x: push_to.x,
                by: slot,
            });
        }
        if let Some(worker) = self.workers.get_mut(seat) {
            worker.feet = target;
        }
    }

    fn jump_worker(&mut self, seat: usize, slot: Slot, events: &mut Vec<GameEvent>) {
        let Some(worker) = self.workers.get(seat) else {
            return;
        };
        let new_head = worker.head().above();
        if worker.state != WorkerState::Grounded
            || !self.field.contains(new_head)
            || self.field.is_occupied(new_head)
        {
            return;
        }
        if let Some(worker) = self.workers.get_mut(seat) {
            worker.feet = worker.feet.above();
            worker.state = WorkerState::Jumping;
            events.push(GameEvent::WorkerJumped { slot });
        }
    }

    fn update_cranes(&mut self, events: &mut Vec<GameEvent>) {
        let width = self.field.width();
        let top = self.field.top_row();
        for crane in &mut self.cranes {
            match crane.state {
                CraneState::Moving => {
                    crane.x += (crane.target_x - crane.x).signum();
                    if crane.x != crane.target_x {
                        continue;
                    }
                    let box_id = self.next_box_id;
                    let cargo = CargoBox {
                        id: box_id,
                        cell: Cell::new(crane.x, top),
                        pattern: crane.pattern,
                        falling: true,
                    };
                    // A2: an occupied top cell (another crane's box this tick, or one still
                    // falling) makes the crane wait above the column instead of overwriting it.
                    if self.field.try_add(cargo) {
                        self.next_box_id = box_id.next();
                        crane.state = CraneState::Dropping;
                        events.push(GameEvent::BoxDropped {
                            crane_id: crane.id,
                            box_id,
                            x: crane.x,
                        });
                    }
                }
                CraneState::Dropping => crane.state = CraneState::Leaving,
                CraneState::Leaving => {
                    let exit_x = crane.exit_side(width).outside_column(width);
                    crane.x += (exit_x - crane.x).signum();
                }
            }
        }
        self.cranes.retain(|crane| {
            crane.state != CraneState::Leaving
                || crane.x != crane.exit_side(width).outside_column(width)
        });
    }

    /// Legacy: falls one cell per tick; a jump hangs for one tick first. Workers do not stand
    /// on each other (they may overlap).
    fn apply_worker_gravity(&mut self, events: &mut Vec<GameEvent>) {
        for (seat, slot) in Slot::all(self.workers.len()).enumerate() {
            let Some(worker) = self.workers.get_mut(seat).filter(|w| w.is_alive()) else {
                continue;
            };
            if worker.state == WorkerState::Jumping {
                worker.state = WorkerState::Falling;
                continue;
            }
            let supported = worker.feet.y == 0 || self.field.is_occupied(worker.feet.below());
            if !supported {
                worker.feet = worker.feet.below();
                worker.state = WorkerState::Falling;
            } else if worker.state == WorkerState::Falling {
                worker.state = WorkerState::Grounded;
                events.push(GameEvent::WorkerLanded {
                    slot,
                    feet: worker.feet,
                });
            }
        }
    }

    fn clear_lines(&mut self, events: &mut Vec<GameEvent>) {
        let rows = self.field.clear_full_rows();
        if rows.is_empty() {
            return;
        }
        let count = u32::try_from(rows.len()).unwrap_or(u32::MAX);
        let points = line_clear_points(count);
        self.score.add(points);
        self.lines_cleared = self.lines_cleared.saturating_add(count);
        self.clearing_ticks = self.clearing_ticks.saturating_add(1);
        events.push(GameEvent::LinesCleared { rows, points });
    }

    fn crush_workers(&mut self, events: &mut Vec<GameEvent>) {
        for (seat, slot) in Slot::all(self.workers.len()).enumerate() {
            let Some(worker) = self.workers.get_mut(seat).filter(|w| w.is_alive()) else {
                continue;
            };
            let crushing_box = self
                .field
                .box_at(worker.feet)
                .or_else(|| self.field.box_at(worker.head()));
            if let Some(box_id) = crushing_box {
                worker.leave_play(WorkerStatus::Crushed);
                events.push(GameEvent::WorkerCrushed { slot, box_id });
            }
        }
    }
}
