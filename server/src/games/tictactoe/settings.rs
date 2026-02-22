use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{ReplayGame, TicTacToeLobbySettings, lobby_details, lobby_settings};
use crate::games::{GameSession, GameSessionConfig, LobbySettings, ReplayMode};
use crate::replay::ReplayRecorder;
use super::session::TicTacToeSessionState;
use super::types::FirstPlayerMode;

pub struct TicTacToeSessionSettings {
    pub field_width: usize,
    pub field_height: usize,
    pub win_count: usize,
    pub first_player_mode: FirstPlayerMode,
}

impl From<&TicTacToeLobbySettings> for TicTacToeSessionSettings {
    fn from(settings: &TicTacToeLobbySettings) -> Self {
        let first_player_mode =
            match crate::proto::tictactoe::FirstPlayerMode::try_from(settings.first_player) {
                Ok(crate::proto::tictactoe::FirstPlayerMode::Host) => FirstPlayerMode::Host,
                Ok(
                    crate::proto::tictactoe::FirstPlayerMode::Random
                    | crate::proto::tictactoe::FirstPlayerMode::Unspecified,
                )
                | Err(_) => FirstPlayerMode::Random,
            };

        Self {
            field_width: settings.field_width as usize,
            field_height: settings.field_height as usize,
            win_count: settings.win_count as usize,
            first_player_mode,
        }
    }
}

impl LobbySettings for TicTacToeLobbySettings {
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

    fn validate_player_count(&self, player_count: usize) -> Result<(), String> {
        if player_count != 2 {
            return Err(format!(
                "TicTacToe requires exactly 2 players, got {}",
                player_count
            ));
        }
        Ok(())
    }

    fn to_proto_details(&self) -> lobby_details::Settings {
        lobby_details::Settings::Tictactoe(*self)
    }

    fn to_proto_info(&self) -> lobby_settings::Settings {
        lobby_settings::Settings::Tictactoe(*self)
    }

    fn game_type(&self) -> ReplayGame {
        ReplayGame::Tictactoe
    }

    fn create_session(
        &self,
        config: &GameSessionConfig,
        seed: u64,
        replay_mode: ReplayMode,
    ) -> Result<GameSession, String> {
        let settings = TicTacToeSessionSettings::from(self);

        let replay_recorder = match replay_mode {
            ReplayMode::Save => {
                let players: Vec<crate::PlayerIdentity> = config
                    .human_players
                    .iter()
                    .map(|p| crate::PlayerIdentity {
                        player_id: p.to_string(),
                        is_bot: false,
                    })
                    .chain(config.bots.keys().map(|b| crate::PlayerIdentity {
                        player_id: b.to_player_id().to_string(),
                        is_bot: true,
                    }))
                    .collect();

                Some(Arc::new(Mutex::new(ReplayRecorder::new(
                    crate::version::VERSION.to_string(),
                    ReplayGame::Tictactoe,
                    seed,
                    Some(lobby_settings::Settings::Tictactoe(*self)),
                    players,
                ))))
            }
            ReplayMode::Discard => None,
        };

        let session_state = TicTacToeSessionState::create(config, &settings, seed, replay_recorder)?;
        Ok(GameSession::TicTacToe(session_state))
    }
}
