use std::sync::Arc;

use tokio::sync::{Mutex, Notify};

use super::game_state::Puzzle2048GameState;
use super::types::{Direction, GameStatus};
use crate::games::broadcaster::GameBroadcaster;
use crate::games::session_config::GameSessionConfig;
use crate::games::session_rng::SessionRng;
use crate::identifiers::ClientId;
use crate::proto::game_service::{GameOverNotification, GameStateUpdate, ScoreEntry};
use crate::proto::puzzle2048::{
    self as proto, Puzzle2048GameEndInfo, Puzzle2048GameEndReason, Puzzle2048InGameCommand,
};
use crate::replay::recorder::ReplayRecorder;
use crate::{InGameCommand, in_game_command};

#[derive(Clone)]
pub struct Puzzle2048SessionState {
    pub session_id: String,
    pub game_state: Arc<Mutex<Puzzle2048GameState>>,
    pub rng: Arc<Mutex<SessionRng>>,
    pub action_notify: Arc<Notify>,
    pub replay_recorder: Option<Arc<Mutex<ReplayRecorder>>>,
    pub player_id: ClientId,
    pub tick: Arc<Mutex<u64>>,
}

impl Puzzle2048SessionState {
    pub fn create(
        config: &GameSessionConfig,
        width: usize,
        height: usize,
        target_value: u32,
        seed: u64,
        replay_recorder: Option<Arc<Mutex<ReplayRecorder>>>,
    ) -> Result<Self, String> {
        if config.human_players.len() != 1 {
            return Err("Puzzle 2048 requires exactly 1 player".to_string());
        }

        let mut rng = SessionRng::new(seed);
        let game_state = Puzzle2048GameState::new(width, height, target_value, &mut rng);

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

pub struct Puzzle2048Session;

impl Puzzle2048Session {
    pub async fn run<B: GameBroadcaster>(
        config: &GameSessionConfig,
        state: &Puzzle2048SessionState,
        broadcaster: &B,
    ) -> GameOverNotification {
        let recipients = config.get_all_recipients();

        loop {
            let (proto_state, status) = {
                let game_state = state.game_state.lock().await;
                let proto = game_state.to_proto();
                let status = game_state.status();
                (proto, status)
            };

            let state_update = GameStateUpdate {
                state: Some(
                    crate::proto::game_service::game_state_update::State::Puzzle2048(proto_state),
                ),
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
        state: &Puzzle2048SessionState,
        client_id: &ClientId,
        command: Puzzle2048InGameCommand,
    ) {
        if client_id != &state.player_id {
            return;
        }

        let Some(cmd) = command.command else {
            return;
        };

        let result = {
            let mut game_state = state.game_state.lock().await;
            let mut rng = state.rng.lock().await;

            match cmd {
                proto::puzzle2048_in_game_command::Command::Move(move_cmd) => {
                    let direction = match move_cmd.direction() {
                        proto::Puzzle2048Direction::Up => Direction::Up,
                        proto::Puzzle2048Direction::Down => Direction::Down,
                        proto::Puzzle2048Direction::Left => Direction::Left,
                        proto::Puzzle2048Direction::Right => Direction::Right,
                        proto::Puzzle2048Direction::Unspecified => return,
                    };
                    let changed = game_state.apply_move(direction, &mut rng);
                    if changed { Ok(()) } else { Err("No change") }
                }
            }
        };

        if result.is_ok() {
            let mut tick = state.tick.lock().await;
            if let Some(ref recorder) = state.replay_recorder {
                let mut recorder = recorder.lock().await;
                if let Some(player_index) = recorder.find_player_index(&client_id.to_string()) {
                    let in_game_command = InGameCommand {
                        command: Some(in_game_command::Command::Puzzle2048(command)),
                    };
                    recorder.record_command(*tick as i64, player_index, in_game_command);
                }
            }
            *tick += 1;
            drop(tick);

            state.action_notify.notify_one();
        }
    }

    pub async fn handle_player_disconnect(state: &Puzzle2048SessionState) {
        let game_state = state.game_state.lock().await;
        if game_state.status() == GameStatus::InProgress {
            // Game ends when player disconnects - handled by game session manager
        }
        drop(game_state);
        state.action_notify.notify_one();
    }

    async fn build_game_over_notification(
        state: &Puzzle2048SessionState,
        won: bool,
    ) -> GameOverNotification {
        let game_state = state.game_state.lock().await;

        let reason = if won {
            Puzzle2048GameEndReason::Won
        } else {
            Puzzle2048GameEndReason::Lost
        };

        let game_end_info = Puzzle2048GameEndInfo {
            reason: reason.into(),
            final_score: game_state.score(),
            highest_tile: game_state.highest_tile(),
            moves_made: game_state.moves_made(),
        };

        let score = game_state.score();

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
                crate::proto::game_service::game_over_notification::GameInfo::Puzzle2048Info(
                    game_end_info,
                ),
            ),
        }
    }
}
