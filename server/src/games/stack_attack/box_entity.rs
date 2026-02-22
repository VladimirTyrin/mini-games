use super::types::Point;

#[derive(Debug, Clone)]
pub struct BoxEntity {
    pub id: u32,
    pub position: Point,
    pub pattern_id: u32,
    pub falling: bool,
}

impl BoxEntity {
    pub fn new(id: u32, position: Point, pattern_id: u32) -> Self {
        Self {
            id,
            position,
            pattern_id,
            falling: true,
        }
    }

    pub fn to_proto(&self) -> crate::proto::stack_attack::Box {
        crate::proto::stack_attack::Box {
            id: self.id,
            x: self.position.x,
            y: self.position.y,
            pattern_id: self.pattern_id,
            falling: self.falling,
        }
    }
}
