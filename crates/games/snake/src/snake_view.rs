use mg_core::{PlayerKind, Slot};

use crate::death_reason::DeathReason;
use crate::direction::Direction;
use crate::point::Point;
use crate::score::Score;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnakeView {
    pub slot: Slot,
    pub player: PlayerKind,
    /// Head first. Kept after death; see [`crate::DeadSnakeBehavior`].
    pub body: Vec<Point>,
    /// Direction of the last move (the starting direction before the first one).
    pub direction: Direction,
    /// Queued turns, oldest first.
    pub queued_turns: Vec<Direction>,
    /// `None` while alive.
    pub death: Option<DeathReason>,
    pub score: Score,
}

impl SnakeView {
    pub fn is_alive(&self) -> bool {
        self.death.is_none()
    }
}
