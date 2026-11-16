use crate::game::StackAttack;
use crate::model::{Cell, Direction, Worker, WorkerState};

impl StackAttack {
    /// True when every living worker is trapped. Nothing can free them then: only workers push
    /// boxes, cranes only raise the walls, rows at the worker's feet and head can never fill
    /// (their column stays empty there) and a clear below shifts the walls and the worker alike.
    /// A crate falling into the pit either crushes the worker or breaks on their head.
    pub(crate) fn all_living_workers_trapped(&self) -> bool {
        let mut living = self.workers.iter().filter(|w| w.is_alive()).peekable();
        living.peek().is_some() && living.all(|worker| self.is_trapped(worker))
    }

    /// A grounded worker who can step neither left nor right, walking or after a jump.
    ///
    /// Only resting boxes count as walls: a crate still falling past the pit must not end the
    /// game for a moment's blockage; once it lands, the check sees it.
    fn is_trapped(&self, worker: &Worker) -> bool {
        if worker.state != WorkerState::Grounded {
            return false;
        }
        let raised = worker.feet.above();
        let raised_head = Worker::head_for(raised);
        let can_jump =
            self.field.contains(raised_head) && !self.field.has_resting_box_at(raised_head);
        [Direction::Left, Direction::Right]
            .into_iter()
            .all(|direction| {
                !self.could_step(worker.feet, direction)
                    && !(can_jump && self.could_step(raised, direction))
            })
    }

    /// Mirrors `move_worker` against resting boxes only.
    fn could_step(&self, feet: Cell, direction: Direction) -> bool {
        let target = feet.shifted_x(direction.dx());
        if !self.field.contains(target) || self.field.has_resting_box_at(Worker::head_for(target)) {
            return false;
        }
        if !self.field.has_resting_box_at(target) {
            return true;
        }
        let push_to = target.shifted_x(direction.dx());
        self.field.contains(push_to)
            && !self.field.has_resting_box_at(push_to)
            && !self.living_worker_at(push_to)
    }
}
