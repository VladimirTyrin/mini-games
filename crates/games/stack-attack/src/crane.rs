use crate::column::Column;
use crate::crane_id::CraneId;
use crate::crane_state::CraneState;
use crate::pattern_id::PatternId;
use crate::side::Side;

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
