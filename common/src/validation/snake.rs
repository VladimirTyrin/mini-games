use crate::proto::snake::SnakeLobbySettings;
use super::ValidateLobbySettings;

impl ValidateLobbySettings for SnakeLobbySettings {
    fn validate(&self, max_players: u32) -> Result<(), String> {
        if self.field_width < 10 || self.field_width > 100 {
            return Err("Field width must be between 10 and 100".to_string());
        }
        if self.field_height < 10 || self.field_height > 100 {
            return Err("Field height must be between 10 and 100".to_string());
        }
        if max_players < 1 || max_players > 10 {
            return Err("Snake supports 1-10 players".to_string());
        }
        if self.tick_interval_ms < 50 || self.tick_interval_ms > 5000 {
            return Err("Tick interval must be between 50ms and 5000ms".to_string());
        }
        if self.max_food_count < 1 || self.max_food_count > 50 {
            return Err("Max food count must be between 1 and 50".to_string());
        }
        if !(0.0..=1.0).contains(&self.food_spawn_probability) {
            return Err("Food spawn probability must be between 0.0 and 1.0".to_string());
        }
        Ok(())
    }
}
