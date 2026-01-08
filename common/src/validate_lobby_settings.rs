pub trait ValidateLobbySettings {
    fn validate(&self, max_players: u32) -> Result<(), String>;
}
