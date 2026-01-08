use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::{
    ReplayGame, SnakeLobbySettings,
    lobby_details, lobby_settings,
};
use crate::games::{
    GameSession, GameSessionConfig, LobbySettings, ReplayMode,
};
use crate::replay::ReplayRecorder;
use super::session::SnakeSessionState;
use super::types::{DeadSnakeBehavior, WallCollisionMode};

pub struct SnakeSessionSettings {
    pub field_width: usize,
    pub field_height: usize,
    pub wall_collision_mode: WallCollisionMode,
    pub dead_snake_behavior: DeadSnakeBehavior,
    pub max_food_count: usize,
    pub food_spawn_probability: f32,
    pub tick_interval: Duration,
}

impl From<&SnakeLobbySettings> for SnakeSessionSettings {
    fn from(settings: &SnakeLobbySettings) -> Self {
        let wall_collision_mode =
            match crate::proto::snake::WallCollisionMode::try_from(settings.wall_collision_mode) {
                Ok(crate::proto::snake::WallCollisionMode::Death)
                | Ok(crate::proto::snake::WallCollisionMode::Unspecified) => WallCollisionMode::Death,
                Ok(crate::proto::snake::WallCollisionMode::WrapAround) => {
                    WallCollisionMode::WrapAround
                }
                _ => WallCollisionMode::Death,
            };
        let dead_snake_behavior =
            match crate::proto::snake::DeadSnakeBehavior::try_from(settings.dead_snake_behavior) {
                Ok(crate::proto::snake::DeadSnakeBehavior::StayOnField) => {
                    DeadSnakeBehavior::StayOnField
                }
                Ok(
                    crate::proto::snake::DeadSnakeBehavior::Disappear
                    | crate::proto::snake::DeadSnakeBehavior::Unspecified,
                )
                | Err(_) => DeadSnakeBehavior::Disappear,
            };

        Self {
            field_width: settings.field_width as usize,
            field_height: settings.field_height as usize,
            wall_collision_mode,
            dead_snake_behavior,
            max_food_count: settings.max_food_count.max(1) as usize,
            food_spawn_probability: settings.food_spawn_probability.clamp(0.001, 1.0),
            tick_interval: Duration::from_millis(settings.tick_interval_ms as u64),
        }
    }
}

impl LobbySettings for SnakeLobbySettings {
    fn validate(&self, max_players: u32) -> Result<(), String> {
        if self.field_width < 10 || self.field_width > 100 {
            return Err("Field width must be between 10 and 100".to_string());
        }
        if self.field_height < 10 || self.field_height > 100 {
            return Err("Field height must be between 10 and 100".to_string());
        }
        if !(1..=10).contains(&max_players) {
            return Err("Snake supports 1-10 players".to_string());
        }
        if self.tick_interval_ms < 50 || self.tick_interval_ms > 5000 {
            return Err("Tick interval must be between 50ms and 5000ms".to_string());
        }
        if self.max_food_count < 1 || self.max_food_count > 50 {
            return Err("Max food count must be between 1 and 50".to_string());
        }
        if !(0.0..=1.0).contains(&self.food_spawn_probability) {
            return Err("Food spawn probability must be between 0.0 and 1.0".to_string());
        }
        Ok(())
    }

    fn validate_player_count(&self, player_count: usize) -> Result<(), String> {
        if player_count < 1 {
            return Err("Snake requires at least 1 player".to_string());
        }
        Ok(())
    }

    fn to_proto_details(&self) -> lobby_details::Settings {
        lobby_details::Settings::Snake(*self)
    }

    fn to_proto_info(&self) -> lobby_settings::Settings {
        lobby_settings::Settings::Snake(*self)
    }

    fn game_type(&self) -> ReplayGame {
        ReplayGame::Snake
    }

    fn create_session(
        &self,
        config: &GameSessionConfig,
        seed: u64,
        replay_mode: ReplayMode,
    ) -> Result<GameSession, String> {
        let settings = SnakeSessionSettings::from(self);

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
                    ReplayGame::Snake,
                    seed,
                    Some(lobby_settings::Settings::Snake(*self)),
                    players,
                ))))
            }
            ReplayMode::Discard => None,
        };

        let session_state = SnakeSessionState::create(config, &settings, seed, replay_recorder);
        Ok(GameSession::Snake(session_state))
    }
}
