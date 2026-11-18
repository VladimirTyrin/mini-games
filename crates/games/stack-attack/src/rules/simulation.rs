use mg_core::Slot;

use crate::game::StackAttack;
use crate::io::Command;
use crate::model::{CargoBox, Cell, CraneState, Direction, WorkerState, WorkerStatus};
use crate::rules::{EndReason, line_clear_points};
use crate::view::GameEvent;

impl StackAttack {
    /// Phase order matters:
    /// * worker actions run before gravity, so a jump-then-move lands on the box it reached;
    /// * head hits are part of the jump action, so a broken box never counts for a line clear;
    /// * line clears lower the workers above a cleared row along with the crates, so a clear
    ///   never shifts a crate into a worker; crush detection runs after them;
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
        } else if self.all_living_workers_trapped() {
            self.end_reason = Some(EndReason::WorkersTrapped);
        }
        self.ticks_survived = self.ticks_survived.next();
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

    pub(super) fn living_worker_at(&self, cell: Cell) -> bool {
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
            // Nor onto one: over a worker's head the crate would drop and crush them this tick.
            if self.living_worker_at(push_to)
                || self.living_worker_at(push_to.below())
                || !self.field.try_move(box_id, push_to)
            {
                return;
            }
            self.boxes_pushed = self.boxes_pushed.saturating_add(1);
            events.push(GameEvent::BoxPushed {
                box_id,
                from_x: target.column(),
                to_x: push_to.column(),
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
        // Grounded alone is not enough: a line clear or a crate pushed away can take the support
        // away after this tick's gravity, and the worker only starts to fall on the next tick.
        if worker.state != WorkerState::Grounded
            || !self.stands_on_something(worker.feet)
            || !self.field.contains(new_head)
            || self.field.has_resting_box_at(new_head)
        {
            return;
        }
        if let Some(worker) = self.workers.get_mut(seat) {
            worker.feet = worker.feet.above();
            worker.state = WorkerState::Jumping;
            events.push(GameEvent::WorkerJumped { slot });
        }
        // Head hit: the rising head breaks the falling crate it rises into and the falling crate
        // right above that one, which this tick's gravity would otherwise drop onto it (the cell
        // below it is empty now). Resolving both here, before cranes and gravity, keeps the hit
        // independent of the gravity scan; a crane only releases after this, so a crate still on
        // its hook is never hit.
        for cell in [new_head, new_head.above()] {
            if let Some(box_id) = self.field.remove_falling_box_at(cell) {
                events.push(GameEvent::BoxBroken {
                    box_id,
                    cell,
                    by: slot,
                });
            }
        }
    }

    fn update_cranes(&mut self, events: &mut Vec<GameEvent>) {
        let width = self.field.width();
        let top = self.field.top_row();
        for crane in &mut self.cranes {
            match crane.state {
                CraneState::Moving => {
                    let was_over_column = crane.x == crane.target_x;
                    crane.x = crane.x.step_towards(crane.target_x);
                    // Arriving over the column, the crane holds the crate for a tick before it
                    // lets go: a crate snapping off the moment the crane stops felt abrupt, and
                    // the pause gives the workers below a moment to see it coming.
                    if crane.x != crane.target_x || !was_over_column {
                        continue;
                    }
                    let box_id = self.next_box_id;
                    let cargo = CargoBox {
                        id: box_id,
                        cell: Cell::new(crane.x.get(), top),
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
                    crane.x = crane.x.step_towards(exit_x);
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
            let supported = self
                .workers
                .get(seat)
                .is_some_and(|worker| self.stands_on_something(worker.feet));
            let Some(worker) = self.workers.get_mut(seat).filter(|w| w.is_alive()) else {
                continue;
            };
            if worker.state == WorkerState::Jumping {
                worker.state = WorkerState::Falling;
                continue;
            }
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

    fn stands_on_something(&self, feet: Cell) -> bool {
        feet.y == 0 || self.field.is_occupied(feet.below())
    }

    fn clear_lines(&mut self, events: &mut Vec<GameEvent>) {
        let rows = self.field.clear_full_rows();
        if rows.is_empty() {
            return;
        }
        for worker in self.workers.iter_mut().filter(|w| w.is_alive()) {
            let rows_below = rows.iter().filter(|row| row.get() < worker.feet.y).count();
            let rows_below = i32::try_from(rows_below).unwrap_or(0);
            worker.feet = Cell::new(worker.feet.x, worker.feet.y - rows_below);
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
