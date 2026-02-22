use std::sync::Arc;

use tokio::sync::{Mutex, Notify};

use super::game_state::{position_from_index, NumbersMatchGameState};
use super::types::{GameStatus, HintMode};
use crate::games::broadcaster::GameBroadcaster;
use crate::games::session_config::GameSessionConfig;
use crate::games::session_rng::SessionRng;
use crate::identifiers::ClientId;
use crate::proto::game_service::{GameOverNotification, GameStateUpdate, ScoreEntry};
use crate::proto::numbers_match::{
    self as proto, NumbersMatchGameEndInfo, NumbersMatchGameEndReason, NumbersMatchInGameCommand,
};
use crate::replay::recorder::ReplayRecorder;
use crate::{InGameCommand, in_game_command};

#[derive(Clone)]
pub struct NumbersMatchSessionState {
    pub session_id: String,
    pub game_state: Arc<Mutex<NumbersMatchGameState>>,
    pub rng: Arc<Mutex<SessionRng>>,
    pub action_notify: Arc<Notify>,
    pub replay_recorder: Option<Arc<Mutex<ReplayRecorder>>>,
    pub player_id: ClientId,
    pub tick: Arc<Mutex<u64>>,
}

impl NumbersMatchSessionState {
    pub fn create(
        config: &GameSessionConfig,
        hint_mode: HintMode,
        seed: u64,
        replay_recorder: Option<Arc<Mutex<ReplayRecorder>>>,
    ) -> Result<Self, String> {
        if config.human_players.len() != 1 {
            return Err("NumbersMatch requires exactly 1 player".to_string());
        }

        let mut rng = SessionRng::new(seed);
        let game_state = NumbersMatchGameState::new(&mut rng, hint_mode);

        let player_id = ClientId::new(config.human_players[0].to_string());

        Ok(Self {
            session_id: config.session_id.clone(),
            game_state: Arc::new(Mutex::new(game_state)),
            rng: Arc::new(Mutex::new(rng)),
            action_notify: Arc::new(Notify::new()),
            replay_recorder,
            player_id,
            tick: Arc::new(Mutex::new(0)),
        })
    }
}

pub struct NumbersMatchSession;

impl NumbersMatchSession {
    pub async fn run<B: GameBroadcaster>(
        config: &GameSessionConfig,
        state: &NumbersMatchSessionState,
        broadcaster: &B,
    ) -> GameOverNotification {
        let recipients = config.get_all_recipients();

        loop {
            let (proto_state, status) = {
                let mut game_state = state.game_state.lock().await;
                let proto = game_state.to_proto();
                let status = game_state.status();
                game_state.take_events();
                (proto, status)
            };

            let state_update = GameStateUpdate {
                state: Some(crate::proto::game_service::game_state_update::State::NumbersMatch(
                    proto_state,
                )),
            };
            broadcaster
                .broadcast_state(state_update, recipients.clone())
                .await;

            match status {
                GameStatus::Won => {
                    return Self::build_game_over_notification(state, true).await;
                }
                GameStatus::Lost => {
                    return Self::build_game_over_notification(state, false).await;
                }
                GameStatus::InProgress => {}
            }

            state.action_notify.notified().await;
        }
    }

    pub async fn handle_command(
        state: &NumbersMatchSessionState,
        client_id: &ClientId,
        command: NumbersMatchInGameCommand,
    ) {
        if client_id != &state.player_id {
            return;
        }

        let Some(cmd) = command.command else {
            return;
        };

        let result = {
            let mut game_state = state.game_state.lock().await;

            match cmd {
                proto::numbers_match_in_game_command::Command::RemovePair(remove) => {
                    let pos1 = position_from_index(remove.first_index);
                    let pos2 = position_from_index(remove.second_index);
                    game_state.remove_pair(pos1, pos2)
                }
                proto::numbers_match_in_game_command::Command::Refill(_) => game_state.refill(),
                proto::numbers_match_in_game_command::Command::RequestHint(_) => {
                    game_state.request_hint().map(|_| ())
                }
            }
        };

        if result.is_ok() {
            let mut tick = state.tick.lock().await;
            if let Some(ref recorder) = state.replay_recorder {
                let mut recorder = recorder.lock().await;
                if let Some(player_index) = recorder.find_player_index(&client_id.to_string()) {
                    let in_game_command = InGameCommand {
                        command: Some(in_game_command::Command::NumbersMatch(command)),
                    };
                    recorder.record_command(*tick as i64, player_index, in_game_command);
                }
            }
            *tick += 1;
            drop(tick);

            state.action_notify.notify_one();
        }
    }

    pub async fn handle_player_disconnect(state: &NumbersMatchSessionState) {
        let game_state = state.game_state.lock().await;
        if game_state.status() == GameStatus::InProgress {
            // Game ends when player disconnects - handled by game session manager
        }
        drop(game_state);
        state.action_notify.notify_one();
    }

    async fn build_game_over_notification(
        state: &NumbersMatchSessionState,
        won: bool,
    ) -> GameOverNotification {
        let game_state = state.game_state.lock().await;

        let reason = if won {
            NumbersMatchGameEndReason::Won
        } else {
            NumbersMatchGameEndReason::Lost
        };

        let game_end_info = NumbersMatchGameEndInfo {
            reason: reason.into(),
            pairs_removed: game_state.pairs_removed(),
            refills_used: game_state.refills_used(),
            hints_used: game_state.hints_used(),
        };

        let score = if won { 1 } else { 0 };

        GameOverNotification {
            scores: vec![ScoreEntry {
                identity: Some(crate::proto::game_service::PlayerIdentity {
                    player_id: state.player_id.to_string(),
                    is_bot: false,
                }),
                score,
            }],
            winner: if won {
                Some(crate::proto::game_service::PlayerIdentity {
                    player_id: state.player_id.to_string(),
                    is_bot: false,
                })
            } else {
                None
            },
            game_info: Some(
                crate::proto::game_service::game_over_notification::GameInfo::NumbersMatchInfo(
                    game_end_info,
                ),
            ),
        }
    }
}
