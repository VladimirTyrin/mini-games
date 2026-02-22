use crate::proto::stack_attack::StackAttackLobbySettings;
use crate::validate_lobby_settings::ValidateLobbySettings;

impl ValidateLobbySettings for StackAttackLobbySettings {
    fn validate(&self, _max_players: u32) -> Result<(), String> {
        Ok(())
    }
}
