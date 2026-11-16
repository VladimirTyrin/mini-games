use crate::crane_id::CraneId;
use crate::crane_state::CraneState;
use crate::pattern_id::PatternId;
use crate::side::Side;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Crane {
    pub id: CraneId,
    /// Column; `-1` and `field_width` are the off-field columns where cranes enter and leave.
    pub x: i32,
    pub target_x: i32,
    pub state: CraneState,
    pub pattern: PatternId,
}

impl Crane {
    /// Legacy rule: the exit depends on the target column, not on the entry side.
    pub fn exit_side(&self, field_width: i32) -> Side {
        if self.target_x < field_width / 2 {
            Side::Left
        } else {
            Side::Right
        }
    }
}
