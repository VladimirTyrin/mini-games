use crate::proto::numbers_match::{HintMode, NumbersMatchLobbySettings};
use crate::validate_lobby_settings::ValidateLobbySettings;

impl ValidateLobbySettings for NumbersMatchLobbySettings {
    fn validate(&self, _max_players: u32) -> Result<(), String> {
        if self.hint_mode() == HintMode::Unspecified {
            return Err("Hint mode must be specified".to_string());
        }
        Ok(())
    }
}
