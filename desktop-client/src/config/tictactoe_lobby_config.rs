use common::config::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct TicTacToeLobbyConfig {
    pub field_width: u32,
    pub field_height: u32,
    pub win_count: u32,
}

impl Validate for TicTacToeLobbyConfig {
    fn validate(&self) -> Result<(), String> {
        if self.field_width < 3 || self.field_height < 3 {
            return Err("TicTacToe field dimensions must be at least 3x3".to_string());
        }
        if self.field_width > 20 || self.field_height > 20 {
            return Err("TicTacToe field dimensions must not exceed 20x20".to_string());
        }
        let min_dimension = self.field_width.min(self.field_height);
        if self.win_count < 3 || self.win_count > min_dimension {
            return Err(format!(
                "win_count must be between 3 and {} (minimum field dimension)",
                min_dimension
            ));
        }
        Ok(())
    }
}

impl Default for TicTacToeLobbyConfig {
    fn default() -> Self {
        Self {
            field_width: 3,
            field_height: 3,
            win_count: 3,
        }
    }
}
