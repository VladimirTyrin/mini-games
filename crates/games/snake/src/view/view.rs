use mg_core::Slot;

use crate::model::{Point, StepCount};
use crate::rules::GameEnd;
use crate::settings::{DeadSnakeBehavior, WallCollisionMode};
use crate::view::SnakeView;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub steps: StepCount,
    pub field_width: u16,
    pub field_height: u16,
    pub tick_interval_ms: u32,
    pub wall_collision_mode: WallCollisionMode,
    pub dead_snake_behavior: DeadSnakeBehavior,
    pub max_food_count: u8,
    /// Indexed by [`Slot::index`].
    pub snakes: Vec<SnakeView>,
    /// In spawn order.
    pub food: Vec<Point>,
    /// The viewer's own seat.
    pub you: Option<Slot>,
    pub end: Option<GameEnd>,
}
