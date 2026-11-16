use mg_core::{SettingRange, SetupError};

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
    pub const FIELD_WIDTH: SettingRange =
        SettingRange::new("field_width", MIN_FIELD_SIZE, MAX_FIELD_SIZE);
    pub const FIELD_HEIGHT: SettingRange =
        SettingRange::new("field_height", MIN_FIELD_SIZE, MAX_FIELD_SIZE);
    pub const MAX_FOOD_COUNT: SettingRange =
        SettingRange::new("max_food_count", MIN_FOOD, MAX_FOOD);
    pub const FOOD_SPAWN_PERMILLE: SettingRange =
        SettingRange::new("food_spawn_permille", MIN_FOOD_PERMILLE, MAX_FOOD_PERMILLE);
    pub const TICK_INTERVAL_MS: SettingRange =
        SettingRange::new("tick_interval_ms", MIN_TICK_MS, MAX_TICK_MS);

    pub fn validate(&self) -> Result<(), SetupError> {
        Self::FIELD_WIDTH.check(self.field_width)?;
        Self::FIELD_HEIGHT.check(self.field_height)?;
        Self::MAX_FOOD_COUNT.check(self.max_food_count)?;
        Self::FOOD_SPAWN_PERMILLE.check(self.food_spawn_permille)?;
        Self::TICK_INTERVAL_MS.check(self.tick_interval_ms)
    }

    /// Bounds of every numeric setting; the only list the UI reads (see
    /// [`mg_core::Game::setting_ranges`]).
    pub fn ranges(&self) -> Vec<SettingRange> {
        vec![
            Self::FIELD_WIDTH,
            Self::FIELD_HEIGHT,
            Self::MAX_FOOD_COUNT,
            Self::FOOD_SPAWN_PERMILLE,
            Self::TICK_INTERVAL_MS,
        ]
    }
}
