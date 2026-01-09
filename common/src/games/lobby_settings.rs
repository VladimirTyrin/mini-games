use crate::{lobby_details, lobby_settings, ReplayGame};
use crate::config::Validate;
use crate::games::{GameSession, GameSessionConfig, ReplayMode};

pub trait LobbySettings: Send + Sync {
    fn validate(&self, max_players: u32) -> Result<(), String>;

    fn validate_player_count(&self, player_count: usize) -> Result<(), String>;

    fn to_proto_details(&self) -> lobby_details::Settings;

    fn to_proto_info(&self) -> lobby_settings::Settings;

    fn game_type(&self) -> ReplayGame;

    fn create_session(
        &self,
        config: &GameSessionConfig,
        seed: u64,
        replay_mode: ReplayMode,
    ) -> Result<GameSession, String>;
}
