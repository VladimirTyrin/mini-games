pub trait ValidateLobbySettings {
    fn validate(&self, max_players: u32) -> Result<(), String>;
}

pub mod snake;
pub mod tictactoe;
