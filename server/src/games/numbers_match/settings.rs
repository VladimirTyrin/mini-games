use std::sync::Arc;
use tokio::sync::Mutex;

use crate::games::game_session::GameSession;
use crate::games::lobby_settings::LobbySettings;
use crate::games::replay_mode::ReplayMode;
use crate::games::session_config::GameSessionConfig;
use crate::proto::game_service::{lobby_details, lobby_settings};
use crate::proto::numbers_match::{HintMode as ProtoHintMode, NumbersMatchLobbySettings};
use crate::proto::replay::Game as ReplayGame;
use crate::replay::recorder::ReplayRecorder;

use super::session::NumbersMatchSessionState;
use super::types::HintMode;

impl LobbySettings for NumbersMatchLobbySettings {
    fn validate(&self, _max_players: u32) -> Result<(), String> {
        if self.hint_mode() == ProtoHintMode::Unspecified {
            return Err("Hint mode must be specified".to_string());
        }
        Ok(())
    }

    fn validate_player_count(&self, player_count: usize) -> Result<(), String> {
        if player_count != 1 {
            return Err("NumbersMatch requires exactly 1 player".to_string());
        }
        Ok(())
    }

    fn to_proto_details(&self) -> lobby_details::Settings {
        lobby_details::Settings::NumbersMatch(*self)
    }

    fn to_proto_info(&self) -> lobby_settings::Settings {
        lobby_settings::Settings::NumbersMatch(*self)
    }

    fn game_type(&self) -> ReplayGame {
        ReplayGame::NumbersMatch
    }

    fn create_session(
        &self,
        config: &GameSessionConfig,
        seed: u64,
        replay_mode: ReplayMode,
    ) -> Result<GameSession, String> {
        let hint_mode = match self.hint_mode() {
            ProtoHintMode::Limited => HintMode::Limited,
            ProtoHintMode::Unlimited => HintMode::Unlimited,
            ProtoHintMode::Disabled => HintMode::Disabled,
            ProtoHintMode::Unspecified => return Err("Hint mode not specified".to_string()),
        };

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
                    ReplayGame::NumbersMatch,
                    seed,
                    Some(lobby_settings::Settings::NumbersMatch(*self)),
                    players,
                ))))
            }
            ReplayMode::Discard => None,
        };

        let session_state =
            NumbersMatchSessionState::create(config, hint_mode, seed, replay_recorder)?;
        Ok(GameSession::NumbersMatch(session_state))
    }
}
