use crate::model::Point;
use crate::rules::DeathReason;

/// What one seat does during the simultaneous move of a tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MovePlan {
    /// Already dead: does not move and keeps its tail.
    Idle,
    Advance {
        target: Point,
        eats_food: bool,
    },
    Die(DeathReason),
}
