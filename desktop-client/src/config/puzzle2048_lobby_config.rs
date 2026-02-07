use common::config::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
pub struct Puzzle2048LobbyConfig {
    pub field_width: u32,
    pub field_height: u32,
    pub target_value: u32,
}

impl Validate for Puzzle2048LobbyConfig {
    fn validate(&self) -> Result<(), String> {
        if self.field_width < 2 || self.field_width > 10 {
            return Err(format!(
                "Field width must be between 2 and 10, got {}",
                self.field_width
            ));
        }
        if self.field_height < 2 || self.field_height > 10 {
            return Err(format!(
                "Field height must be between 2 and 10, got {}",
                self.field_height
            ));
        }
        if self.target_value < 8 {
            return Err(format!(
                "Target value must be at least 8, got {}",
                self.target_value
            ));
        }
        if !self.target_value.is_power_of_two() {
            return Err(format!(
                "Target value must be a power of 2, got {}",
                self.target_value
            ));
        }
        Ok(())
    }
}

impl Default for Puzzle2048LobbyConfig {
    fn default() -> Self {
        Self {
            field_width: 4,
            field_height: 4,
            target_value: 2048,
        }
    }
}
