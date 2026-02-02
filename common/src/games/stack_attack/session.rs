use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::interval;

use crate::{
    BotId, ClientId, GameOverNotification, GameStateUpdate, PlayerIdentity, PlayerId, ScoreEntry,
    game_over_notification, game_state_update,
    proto::stack_attack::{
        StackAttackGameEndInfo, StackAttackGameEndReason, StackAttackInGameCommand,
        stack_attack_in_game_command, HorizontalDirection as ProtoDirection,
    },
};
use crate::games::{BotType, GameBroadcaster, GameSessionConfig, SessionRng};
use crate::replay::ReplayRecorder;

use super::game_state::StackAttackGameState;
use super::settings::{StackAttackSessionSettings, TICK_INTERVAL_MS};
use super::types::HorizontalDirection;

#[derive(Clone)]
pub struct StackAttackSessionState {
    pub session_id: String,
    pub game_state: Arc<Mutex<StackAttackGameState>>,
    pub tick: Arc<Mutex<u64>>,
    pub rng: Arc<Mutex<SessionRng>>,
    pub bots: HashMap<BotId, BotType>,
    pub replay_recorder: Option<Arc<Mutex<ReplayRecorder>>>,
    pub start_time: std::time::Instant,
}

impl StackAttackSessionState {
    pub fn create(
        config: &GameSessionConfig,
        _settings: &StackAttackSessionSettings,
        seed: u64,
        replay_recorder: Option<Arc<Mutex<ReplayRecorder>>>,
    ) -> Self {
        let rng = SessionRng::new(seed);

        let mut players: Vec<PlayerId> = config.human_players.clone();
        for bot_id in config.bots.keys() {
            players.push(bot_id.to_player_id());
        }

        let game_state = StackAttackGameState::new(&players);

        Self {
            session_id: config.session_id.clone(),
            game_state: Arc::new(Mutex::new(game_state)),
            tick: Arc::new(Mutex::new(0)),
            rng: Arc::new(Mutex::new(rng)),
            bots: config.bots.clone(),
            replay_recorder,
            start_time: std::time::Instant::now(),
        }
    }
}

pub struct StackAttackSession;

impl StackAttackSession {
    pub async fn run(
        config: GameSessionConfig,
        session_state: StackAttackSessionState,
        broadcaster: impl GameBroadcaster,
    ) -> GameOverNotification {
        let tick_duration = std::time::Duration::from_millis(TICK_INTERVAL_MS as u64);
        let mut tick_interval_timer = interval(tick_duration);

        loop {
            tick_interval_timer.tick().await;

            let mut game_state = session_state.game_state.lock().await;
            let mut rng = session_state.rng.lock().await;

            let _events = game_state.update(&mut rng);
            drop(rng);

            let mut tick_value = session_state.tick.lock().await;
            *tick_value += 1;
            let current_tick = *tick_value;
            drop(tick_value);

            let proto_state = game_state.to_proto(current_tick, &session_state.bots);
            let game_over = game_state.is_game_over();
            drop(game_state);

            let recipients = config.get_all_recipients();
            let state_update = GameStateUpdate {
                state: Some(game_state_update::State::StackAttack(proto_state)),
            };
            broadcaster.broadcast_state(state_update, recipients).await;

            if game_over {
                break;
            }
        }

        build_game_over_notification(&session_state).await
    }

    pub async fn handle_command(
        state: &StackAttackSessionState,
        client_id: &ClientId,
        command: StackAttackInGameCommand,
    ) {
        let player_id = PlayerId::new(client_id.to_string());

        match command.command {
            Some(stack_attack_in_game_command::Command::Move(move_cmd)) => {
                let direction = match ProtoDirection::try_from(move_cmd.direction) {
                    Ok(ProtoDirection::Left) => HorizontalDirection::Left,
                    Ok(ProtoDirection::Right) => HorizontalDirection::Right,
                    _ => return,
                };

                let mut game_state = state.game_state.lock().await;
                let _events = game_state.handle_move(&player_id, direction);
            }
            Some(stack_attack_in_game_command::Command::Jump(_)) => {
                let mut game_state = state.game_state.lock().await;
                let _events = game_state.handle_jump(&player_id);
            }
            None => {}
        }
    }

    pub async fn handle_player_disconnect(state: &StackAttackSessionState) {
        let mut game_state = state.game_state.lock().await;
        game_state.handle_player_disconnect();
    }
}

async fn build_game_over_notification(session_state: &StackAttackSessionState) -> GameOverNotification {
    let game_state = session_state.game_state.lock().await;

    let scores: Vec<ScoreEntry> = game_state
        .workers
        .keys()
        .map(|id| {
            let is_bot = session_state
                .bots
                .iter()
                .any(|(bot_id, _)| bot_id.to_player_id() == *id);

            ScoreEntry {
                identity: Some(PlayerIdentity {
                    player_id: id.to_string(),
                    is_bot,
                }),
                score: game_state.score,
            }
        })
        .collect();

    let survival_time = session_state.start_time.elapsed().as_secs() as u32;

    let reason = game_state
        .game_over_reason
        .map(|r| r.to_proto())
        .unwrap_or(StackAttackGameEndReason::Unspecified);

    GameOverNotification {
        scores,
        winner: None,
        game_info: Some(game_over_notification::GameInfo::StackAttackInfo(
            StackAttackGameEndInfo {
                reason: reason as i32,
                total_score: game_state.score,
                lines_cleared: game_state.lines_cleared,
                boxes_pushed: game_state.boxes_pushed,
                survival_time_seconds: survival_time,
            },
        )),
    }
}
