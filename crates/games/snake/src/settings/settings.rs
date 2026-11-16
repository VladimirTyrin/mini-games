use mg_core::{Capped, GameSettings, SettingRange, SettingsError};

use crate::settings::{DeadSnakeBehavior, RawSettings, WallCollisionMode};

pub type FieldSize = Capped<10, 100>;
pub type FoodCount = Capped<1, 50>;
/// Chance per tick, in 1/1000, that one food item spawns while below the food cap; 1000 spawns
/// every tick.
pub type FoodSpawnPermille = Capped<1, 1000>;
pub type TickIntervalMs = Capped<50, 5000>;

pub const MIN_SEATS: u8 = 1;
pub const MAX_SEATS: u8 = 10;

// Start positions put every seat in its own column (see `start_body`).
const _: () = assert!(FieldSize::MIN >= MAX_SEATS as u32);

/// Lobby settings, valid by construction (see [`GameSettings::parse`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Settings {
    pub field_width: FieldSize,
    pub field_height: FieldSize,
    pub wall_collision_mode: WallCollisionMode,
    pub dead_snake_behavior: DeadSnakeBehavior,
    pub max_food_count: FoodCount,
    pub food_spawn_permille: FoodSpawnPermille,
    pub tick_interval_ms: TickIntervalMs,
}

impl Default for Settings {
    /// Mirrors the web client's `getDefaultSnakeSettings`.
    fn default() -> Self {
        Self {
            field_width: FieldSize::constant::<15>(),
            field_height: FieldSize::constant::<15>(),
            wall_collision_mode: WallCollisionMode::WrapAround,
            dead_snake_behavior: DeadSnakeBehavior::Disappear,
            max_food_count: FoodCount::constant::<1>(),
            food_spawn_permille: FoodSpawnPermille::constant::<1000>(),
            tick_interval_ms: TickIntervalMs::constant::<200>(),
        }
    }
}

impl GameSettings for Settings {
    type Raw = RawSettings;

    fn parse(raw: RawSettings) -> Result<Self, SettingsError> {
        Ok(Self {
            field_width: FieldSize::new(raw.field_width)
                .map_err(SettingsError::at("field_width"))?,
            field_height: FieldSize::new(raw.field_height)
                .map_err(SettingsError::at("field_height"))?,
            wall_collision_mode: raw.wall_collision_mode,
            dead_snake_behavior: raw.dead_snake_behavior,
            max_food_count: FoodCount::new(raw.max_food_count)
                .map_err(SettingsError::at("max_food_count"))?,
            food_spawn_permille: FoodSpawnPermille::new(raw.food_spawn_permille)
                .map_err(SettingsError::at("food_spawn_permille"))?,
            tick_interval_ms: TickIntervalMs::new(raw.tick_interval_ms)
                .map_err(SettingsError::at("tick_interval_ms"))?,
        })
    }

    fn to_raw(&self) -> RawSettings {
        RawSettings {
            field_width: self.field_width.get(),
            field_height: self.field_height.get(),
            wall_collision_mode: self.wall_collision_mode,
            dead_snake_behavior: self.dead_snake_behavior,
            max_food_count: self.max_food_count.get(),
            food_spawn_permille: self.food_spawn_permille.get(),
            tick_interval_ms: self.tick_interval_ms.get(),
        }
    }

    fn ranges(_raw: &RawSettings) -> Vec<SettingRange> {
        vec![
            FieldSize::range("field_width"),
            FieldSize::range("field_height"),
            FoodCount::range("max_food_count"),
            FoodSpawnPermille::range("food_spawn_permille"),
            TickIntervalMs::range("tick_interval_ms"),
        ]
    }
}
