mod dead_snake_behavior;
mod settings;
mod wall_collision_mode;

pub use dead_snake_behavior::DeadSnakeBehavior;
pub use settings::{
    MAX_FIELD_SIZE, MAX_FOOD, MAX_FOOD_PERMILLE, MAX_SEATS, MAX_TICK_MS, MIN_FIELD_SIZE, MIN_FOOD,
    MIN_FOOD_PERMILLE, MIN_SEATS, MIN_TICK_MS, Settings,
};
pub use wall_collision_mode::WallCollisionMode;
