use mg_core::SetupError;

use crate::settings::{DeadSnakeBehavior, WallCollisionMode};

pub const MIN_FIELD_SIZE: u32 = 10;
pub const MAX_FIELD_SIZE: u32 = 100;
pub const MIN_TICK_MS: u32 = 50;
pub const MAX_TICK_MS: u32 = 5000;
pub const MIN_FOOD: u32 = 1;
pub const MAX_FOOD: u32 = 50;
pub const MIN_FOOD_PERMILLE: u32 = 1;
/// 1000 per mille: food spawns every tick while below the cap.
pub const MAX_FOOD_PERMILLE: u32 = 1000;
pub const MIN_SEATS: u8 = 1;
pub const MAX_SEATS: u8 = 10;

// Start positions put every seat in its own column (see `start_body`).
const _: () = assert!(MIN_FIELD_SIZE >= MAX_SEATS as u32);

/// Raw lobby settings; [`mg_core::Game::new`] validates every field against the limits above.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Settings {
    pub field_width: u32,
    pub field_height: u32,
    pub wall_collision_mode: WallCollisionMode,
    pub dead_snake_behavior: DeadSnakeBehavior,
    pub max_food_count: u32,
    /// Chance per tick, in 1/1000, that one food item spawns while below `max_food_count`.
    pub food_spawn_permille: u32,
    pub tick_interval_ms: u32,
}

impl Default for Settings {
    /// Mirrors the web client's `getDefaultSnakeSettings`.
    fn default() -> Self {
        Self {
            field_width: 15,
            field_height: 15,
            wall_collision_mode: WallCollisionMode::WrapAround,
            dead_snake_behavior: DeadSnakeBehavior::Disappear,
            max_food_count: 1,
            food_spawn_permille: 1000,
            tick_interval_ms: 200,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), SetupError> {
        SetupError::check_range(
            "field_width",
            self.field_width,
            MIN_FIELD_SIZE,
            MAX_FIELD_SIZE,
        )?;
        SetupError::check_range(
            "field_height",
            self.field_height,
            MIN_FIELD_SIZE,
            MAX_FIELD_SIZE,
        )?;
        SetupError::check_range("max_food_count", self.max_food_count, MIN_FOOD, MAX_FOOD)?;
        SetupError::check_range(
            "food_spawn_permille",
            self.food_spawn_permille,
            MIN_FOOD_PERMILLE,
            MAX_FOOD_PERMILLE,
        )?;
        SetupError::check_range(
            "tick_interval_ms",
            self.tick_interval_ms,
            MIN_TICK_MS,
            MAX_TICK_MS,
        )
    }
}
