use crate::PlayerId;

use super::field::Field;
use super::types::{HorizontalDirection, MoveResult, Point, WorkerState};

pub const WORKER_HEIGHT: i32 = 2;

#[derive(Debug, Clone)]
pub struct Worker {
    pub player_id: PlayerId,
    pub position: Point,
    pub state: WorkerState,
    pub color_index: u32,
    pub alive: bool,
}

impl Worker {
    pub fn new(player_id: PlayerId, position: Point, color_index: u32) -> Self {
        Self {
            player_id,
            position,
            state: WorkerState::Grounded,
            color_index,
            alive: true,
        }
    }

    pub fn head_y(&self) -> i32 {
        self.position.y + WORKER_HEIGHT - 1
    }

    pub fn try_move(&mut self, direction: HorizontalDirection, field: &mut Field) -> MoveResult {
        let new_x = self.position.x + direction.dx();

        if !field.is_valid_x(new_x) {
            return MoveResult::Blocked;
        }

        let head_y = self.head_y();
        if field.is_occupied(new_x, head_y) {
            return MoveResult::Blocked;
        }

        if let Some(box_id) = field.get_box_id_at(new_x, self.position.y) {
            let push_target_x = new_x + direction.dx();

            if !field.is_valid_x(push_target_x) {
                return MoveResult::Blocked;
            }

            if field.is_occupied(push_target_x, self.position.y) {
                return MoveResult::Blocked;
            }

            field.move_box(box_id, push_target_x);
            self.position.x = new_x;
            return MoveResult::PushedBox(box_id);
        }

        self.position.x = new_x;
        MoveResult::Moved
    }

    pub fn jump(&mut self, field: &Field) -> bool {
        if self.state != WorkerState::Grounded {
            return false;
        }

        let new_head_y = self.head_y() + 1;

        if new_head_y >= field.height() as i32 {
            return false;
        }

        if field.is_occupied(self.position.x, new_head_y) {
            return false;
        }

        self.position.y += 1;
        self.state = WorkerState::Jumping;
        true
    }

    pub fn apply_gravity(&mut self, field: &Field) -> bool {
        if self.state == WorkerState::Jumping {
            self.state = WorkerState::Falling;
            return false;
        }

        if self.position.y == 0 {
            if self.state == WorkerState::Falling {
                self.state = WorkerState::Grounded;
                return true;
            }
            return false;
        }

        let below_y = self.position.y - 1;
        if field.is_occupied(self.position.x, below_y) {
            if self.state == WorkerState::Falling {
                self.state = WorkerState::Grounded;
                return true;
            }
            return false;
        }

        self.position.y = below_y;
        self.state = WorkerState::Falling;
        false
    }

    pub fn to_proto(&self, is_bot: bool) -> crate::proto::stack_attack::Worker {
        crate::proto::stack_attack::Worker {
            player_id: self.player_id.to_string(),
            is_bot,
            x: self.position.x,
            y: self.position.y,
            state: self.state.to_proto() as i32,
            color_index: self.color_index,
            alive: self.alive,
        }
    }
}
