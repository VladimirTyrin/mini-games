use crate::model::{Column, CraneId, CraneState, PatternId, Side};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Crane {
    pub id: CraneId,
    pub x: Column,
    pub target_x: Column,
    pub state: CraneState,
    pub pattern: PatternId,
}

impl Crane {
    /// Legacy rule: the exit depends on the target column, not on the entry side.
    pub fn exit_side(&self, field_width: i32) -> Side {
        if self.target_x.get() < field_width / 2 {
            Side::Left
        } else {
            Side::Right
        }
    }
}
