use common::config::Validate;
use common::{DeadSnakeBehavior, WallCollisionMode};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct SnakeLobbyConfig {
    pub max_players: u32,
    pub field_width: u32,
    pub field_height: u32,
    pub wall_collision_mode: WallCollisionMode,
    pub dead_snake_behavior: DeadSnakeBehavior,
    pub tick_interval_ms: u32,
    pub max_food_count: u32,
    pub food_spawn_probability: f32,
}

impl Validate for SnakeLobbyConfig {
    fn validate(&self) -> Result<(), String> {
        if self.max_players == 0 {
            return Err("max_players must be greater than 0".to_string());
        }
        if self.max_players > 16 {
            return Err("max_players must not exceed 16".to_string());
        }
        if self.field_width < 5 || self.field_height < 5 {
            return Err("field dimensions must be at least 5x5".to_string());
        }
        if self.field_width > 50 || self.field_height > 50 {
            return Err("field dimensions must not exceed 50x50".to_string());
        }
        if self.tick_interval_ms < 50 {
            return Err("tick_interval_ms must be at least 50".to_string());
        }
        if self.tick_interval_ms > 1000 {
            return Err("tick_interval_ms must not exceed 1000".to_string());
        }
        if self.max_food_count < 1 {
            return Err("max_food_count must be at least 1".to_string());
        }
        if self.food_spawn_probability <= 0.0 || self.food_spawn_probability > 1.0 {
            return Err("food_spawn_probability must be greater than 0 and at most 1".to_string());
        }
        Ok(())
    }
}

impl Default for SnakeLobbyConfig {
    fn default() -> Self {
        Self {
            max_players: 4,
            field_width: 15,
            field_height: 15,
            wall_collision_mode: WallCollisionMode::WrapAround,
            dead_snake_behavior: DeadSnakeBehavior::Disappear,
            tick_interval_ms: 200,
            max_food_count: 1,
            food_spawn_probability: 1.0,
        }
    }
}
