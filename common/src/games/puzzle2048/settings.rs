use std::sync::Arc;
use tokio::sync::Mutex;

use crate::games::game_session::GameSession;
use crate::games::lobby_settings::LobbySettings;
use crate::games::replay_mode::ReplayMode;
use crate::games::session_config::GameSessionConfig;
use crate::proto::game_service::{lobby_details, lobby_settings};
use crate::proto::puzzle2048::Puzzle2048LobbySettings;
use crate::proto::replay::Game as ReplayGame;
use crate::replay::recorder::ReplayRecorder;

use super::session::Puzzle2048SessionState;

fn validate_settings(settings: &Puzzle2048LobbySettings) -> Result<(), String> {
    if settings.field_width < 2 || settings.field_width > 10 {
        return Err(format!(
            "Field width must be between 2 and 10, got {}",
            settings.field_width
        ));
    }
    if settings.field_height < 2 || settings.field_height > 10 {
        return Err(format!(
            "Field height must be between 2 and 10, got {}",
            settings.field_height
        ));
    }
    if settings.target_value < 8 {
        return Err(format!(
            "Target value must be at least 8, got {}",
            settings.target_value
        ));
    }
    if !settings.target_value.is_power_of_two() {
        return Err(format!(
            "Target value must be a power of 2, got {}",
            settings.target_value
        ));
    }
    Ok(())
}

impl LobbySettings for Puzzle2048LobbySettings {
    fn validate(&self, _max_players: u32) -> Result<(), String> {
        validate_settings(self)
    }

    fn validate_player_count(&self, player_count: usize) -> Result<(), String> {
        if player_count != 1 {
            return Err("Puzzle 2048 requires exactly 1 player".to_string());
        }
        Ok(())
    }

    fn to_proto_details(&self) -> lobby_details::Settings {
        lobby_details::Settings::Puzzle2048(*self)
    }

    fn to_proto_info(&self) -> lobby_settings::Settings {
        lobby_settings::Settings::Puzzle2048(*self)
    }

    fn game_type(&self) -> ReplayGame {
        ReplayGame::Puzzle2048
    }

    fn create_session(
        &self,
        config: &GameSessionConfig,
        seed: u64,
        replay_mode: ReplayMode,
    ) -> Result<GameSession, String> {
        let replay_recorder = match replay_mode {
            ReplayMode::Save => {
                let players: Vec<crate::PlayerIdentity> = config
                    .human_players
                    .iter()
                    .map(|p| crate::PlayerIdentity {
                        player_id: p.to_string(),
                        is_bot: false,
                    })
                    .collect();

                Some(Arc::new(Mutex::new(ReplayRecorder::new(
                    crate::version::VERSION.to_string(),
                    ReplayGame::Puzzle2048,
                    seed,
                    Some(lobby_settings::Settings::Puzzle2048(*self)),
                    players,
                ))))
            }
            ReplayMode::Discard => None,
        };

        let session_state = Puzzle2048SessionState::create(
            config,
            self.field_width as usize,
            self.field_height as usize,
            self.target_value,
            seed,
            replay_recorder,
        )?;
        Ok(GameSession::Puzzle2048(session_state))
    }
}
