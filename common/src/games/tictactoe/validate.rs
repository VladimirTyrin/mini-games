use crate::proto::tictactoe::TicTacToeLobbySettings;
use crate::validate_lobby_settings::ValidateLobbySettings;

impl ValidateLobbySettings for TicTacToeLobbySettings {
    fn validate(&self, max_players: u32) -> Result<(), String> {
        if max_players != 2 {
            return Err("TicTacToe requires exactly 2 players".to_string());
        }
        if self.field_width < 3 || self.field_width > 20 {
            return Err("Field width must be between 3 and 20".to_string());
        }
        if self.field_height < 3 || self.field_height > 20 {
            return Err("Field height must be between 3 and 20".to_string());
        }
        if self.win_count < 3 {
            return Err("Win count must be at least 3".to_string());
        }
        let min_dimension = self.field_width.min(self.field_height);
        if self.win_count > min_dimension {
            return Err(format!(
                "Win count ({}) cannot exceed minimum dimension ({})",
                self.win_count, min_dimension
            ));
        }
        Ok(())
    }
}
