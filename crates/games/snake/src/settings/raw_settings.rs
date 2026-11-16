use mg_core::GameSettings;

use crate::settings::{DeadSnakeBehavior, Settings, WallCollisionMode};

/// [`Settings`] as clients and replay files send them, unchecked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RawSettings {
    pub field_width: u32,
    pub field_height: u32,
    pub wall_collision_mode: WallCollisionMode,
    pub dead_snake_behavior: DeadSnakeBehavior,
    pub max_food_count: u32,
    pub food_spawn_permille: u32,
    pub tick_interval_ms: u32,
}

impl Default for RawSettings {
    fn default() -> Self {
        Settings::default().to_raw()
    }
}
