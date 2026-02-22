use crate::proto::puzzle2048::Puzzle2048LobbySettings;
use crate::validate_lobby_settings::ValidateLobbySettings;

impl ValidateLobbySettings for Puzzle2048LobbySettings {
    fn validate(&self, _max_players: u32) -> Result<(), String> {
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
