use mg_core::Slot;

use crate::cell_content::CellContent;
use crate::death_reason::DeathReason;
use crate::move_plan::MovePlan;
use crate::point::Point;
use crate::seat_state::SeatState;
use crate::snake::Snake;

impl Snake {
    /// All heads move at once:
    /// 1. every living snake consumes one queued turn and picks its target cell;
    /// 2. leaving the field in `Death` mode kills;
    /// 3. two or more heads aiming at the same cell all die (nobody eats);
    /// 4. a target occupied by a solid body kills, unless it is the tail of a snake that moves
    ///    this tick without growing (that tail is vacated). Swapping cells is covered here: the
    ///    other snake's head cell stays occupied by its neck. Resolved to a fixed point, so a
    ///    ring of snakes chasing each other's tails survives, while the tail of a snake that
    ///    dies stays put;
    /// 5. tails are vacated before heads are placed.
    pub(crate) fn advance(&mut self) {
        if !self.is_running() {
            return;
        }
        self.steps = self.steps.next();
        for seat in self.seats.iter_mut().filter(|seat| seat.is_alive()) {
            seat.consume_queued_turn();
        }
        let mut plans: Vec<MovePlan> = self.seats.iter().map(|seat| self.plan(seat)).collect();
        kill_head_on_collisions(&mut plans);
        self.kill_body_collisions(&mut plans);
        self.apply_plans(&plans);
    }

    fn plan(&self, seat: &SeatState) -> MovePlan {
        if !seat.is_alive() {
            return MovePlan::Idle;
        }
        let target = seat
            .head()
            .and_then(|head| self.field.neighbour(head, seat.direction));
        match target {
            None => MovePlan::Die(DeathReason::WallCollision),
            Some(target) => MovePlan::Advance {
                target,
                eats_food: self.grid.get(target) == Some(CellContent::Food),
            },
        }
    }

    fn kill_body_collisions(&self, plans: &mut [MovePlan]) {
        // Each pass kills at least one snake or stops, so this ends after at most seats + 1 passes.
        loop {
            let mut changed = false;
            for index in 0..plans.len() {
                let Some(MovePlan::Advance { target, .. }) = plans.get(index).copied() else {
                    continue;
                };
                let Some(CellContent::Body(owner)) = self.grid.get(target) else {
                    continue;
                };
                let owner_keeps_length = matches!(
                    plans.get(owner.index()),
                    Some(MovePlan::Advance {
                        eats_food: false,
                        ..
                    })
                );
                let is_owner_tail = self.seat(owner).and_then(SeatState::tail) == Some(target);
                if is_owner_tail && owner_keeps_length {
                    continue;
                }
                let reason = if owner.index() == index {
                    DeathReason::SelfCollision
                } else {
                    DeathReason::SnakeCollision
                };
                if let Some(plan) = plans.get_mut(index) {
                    *plan = MovePlan::Die(reason);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }

    fn apply_plans(&mut self, plans: &[MovePlan]) {
        for (seat, plan) in self.seats.iter_mut().zip(plans) {
            if let MovePlan::Advance {
                eats_food: false, ..
            } = plan
                && let Some(tail) = seat.body.pop_back()
            {
                self.grid.set(tail, CellContent::Empty);
            }
        }
        for ((slot, seat), plan) in Slot::all(self.seats.len())
            .zip(self.seats.iter_mut())
            .zip(plans)
        {
            if let MovePlan::Advance { target, eats_food } = *plan {
                if eats_food {
                    self.food.retain(|&food| food != target);
                    seat.score.increment();
                }
                seat.body.push_front(target);
                self.grid.set(target, CellContent::Body(slot));
            }
        }
        let mut first_death = None;
        for (slot, plan) in Slot::all(plans.len()).zip(plans) {
            if let MovePlan::Die(reason) = *plan {
                self.kill(slot, reason);
                first_death.get_or_insert(reason);
            }
        }
        if let Some(reason) = first_death {
            self.check_end(reason);
        }
    }
}

fn kill_head_on_collisions(plans: &mut [MovePlan]) {
    let targets: Vec<Option<Point>> = plans
        .iter()
        .map(|plan| match plan {
            MovePlan::Advance { target, .. } => Some(*target),
            MovePlan::Idle | MovePlan::Die(_) => None,
        })
        .collect();
    for (index, plan) in plans.iter_mut().enumerate() {
        let MovePlan::Advance { target, .. } = *plan else {
            continue;
        };
        let contested = targets
            .iter()
            .enumerate()
            .any(|(other, other_target)| other != index && *other_target == Some(target));
        if contested {
            *plan = MovePlan::Die(DeathReason::SnakeCollision);
        }
    }
}
