mod dead_snake_behavior;
mod raw_settings;
mod settings;
mod wall_collision_mode;

pub use dead_snake_behavior::DeadSnakeBehavior;
pub use raw_settings::RawSettings;
pub use settings::{
    FieldSize, FoodCount, FoodSpawnPermille, MAX_SEATS, MIN_SEATS, Settings, TickIntervalMs,
};
pub use wall_collision_mode::WallCollisionMode;
