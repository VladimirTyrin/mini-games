use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::{
    ReplayGame, lobby_details, lobby_settings,
    proto::stack_attack::StackAttackLobbySettings,
};
use crate::games::{GameSession, GameSessionConfig, ReplayMode};
use crate::games::lobby_settings::LobbySettings;
use crate::replay::ReplayRecorder;

pub const FIELD_WIDTH: u32 = 15;
pub const FIELD_HEIGHT: u32 = 10;
pub const TICK_INTERVAL_MS: u32 = 200;

pub const MIN_PLAYERS: usize = 1;
pub const MAX_PLAYERS: usize = 4;

#[derive(Debug, Clone, Default)]
pub struct StackAttackSessionSettings;

impl StackAttackSessionSettings {
    pub fn field_width(&self) -> u32 {
        FIELD_WIDTH
    }

    pub fn field_height(&self) -> u32 {
        FIELD_HEIGHT
    }

    pub fn tick_interval(&self) -> Duration {
        Duration::from_millis(TICK_INTERVAL_MS as u64)
    }
}

impl LobbySettings for StackAttackLobbySettings {
    fn validate(&self, max_players: u32) -> Result<(), String> {
        if !(MIN_PLAYERS as u32..=MAX_PLAYERS as u32).contains(&max_players) {
            return Err(format!(
                "Stack Attack supports {}-{} players",
                MIN_PLAYERS, MAX_PLAYERS
            ));
        }
        Ok(())
    }

    fn validate_player_count(&self, player_count: usize) -> Result<(), String> {
        if player_count < MIN_PLAYERS {
            return Err(format!(
                "Stack Attack requires at least {} player(s)",
                MIN_PLAYERS
            ));
        }
        if player_count > MAX_PLAYERS {
            return Err(format!(
                "Stack Attack allows at most {} players",
                MAX_PLAYERS
            ));
        }
        Ok(())
    }

    fn to_proto_details(&self) -> lobby_details::Settings {
        lobby_details::Settings::StackAttack(*self)
    }

    fn to_proto_info(&self) -> lobby_settings::Settings {
        lobby_settings::Settings::StackAttack(*self)
    }

    fn game_type(&self) -> ReplayGame {
        ReplayGame::StackAttack
    }

    fn create_session(
        &self,
        config: &GameSessionConfig,
        seed: u64,
        replay_mode: ReplayMode,
    ) -> Result<GameSession, String> {
        use super::session::StackAttackSessionState;

        let settings = StackAttackSessionSettings;

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
                    ReplayGame::StackAttack,
                    seed,
                    Some(lobby_settings::Settings::StackAttack(*self)),
                    players,
                ))))
            }
            ReplayMode::Discard => None,
        };

        let state = StackAttackSessionState::create(config, &settings, seed, replay_recorder);
        Ok(GameSession::StackAttack(state))
    }
}
