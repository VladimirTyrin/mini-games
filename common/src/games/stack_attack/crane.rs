use super::types::CraneState;

pub enum CraneAction {
    DropBox,
    Finished,
}

#[derive(Debug, Clone)]
pub struct Crane {
    pub id: u32,
    pub x: i32,
    pub target_x: i32,
    pub state: CraneState,
    pub box_pattern_id: u32,
    field_width: u32,
}

impl Crane {
    pub fn new(id: u32, start_x: i32, target_x: i32, pattern_id: u32, field_width: u32) -> Self {
        Self {
            id,
            x: start_x,
            target_x,
            state: CraneState::Moving,
            box_pattern_id: pattern_id,
            field_width,
        }
    }

    pub fn update(&mut self) -> Option<CraneAction> {
        match self.state {
            CraneState::Moving => {
                if self.x < self.target_x {
                    self.x += 1;
                } else if self.x > self.target_x {
                    self.x -= 1;
                }

                if self.x == self.target_x {
                    self.state = CraneState::Dropping;
                    return Some(CraneAction::DropBox);
                }
                None
            }
            CraneState::Dropping => {
                self.state = CraneState::Leaving;
                None
            }
            CraneState::Leaving => {
                let exit_x = if self.target_x < (self.field_width as i32) / 2 {
                    -1
                } else {
                    self.field_width as i32
                };

                if self.x < exit_x {
                    self.x += 1;
                } else if self.x > exit_x {
                    self.x -= 1;
                }

                if self.x == exit_x {
                    return Some(CraneAction::Finished);
                }
                None
            }
        }
    }

    pub fn to_proto(&self) -> crate::proto::stack_attack::Crane {
        crate::proto::stack_attack::Crane {
            id: self.id,
            x: self.x,
            target_x: self.target_x,
            state: self.state.to_proto() as i32,
            box_pattern_id: self.box_pattern_id,
        }
    }
}
