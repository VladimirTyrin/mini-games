use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;

use crate::{
    BotId, ClientId, GameOverNotification, GameStateUpdate, PlayerIdentity, PlayerId, ScoreEntry,
    SnakePosition, InGameCommand, in_game_command, game_over_notification, game_state_update, log,
    proto::snake::{
        Direction as ProtoDirection, SnakeGameEndInfo, SnakeGameEndReason,
        SnakeGameState as ProtoSnakeGameState, SnakeInGameCommand, snake_in_game_command,
        TurnCommand,
    },
};
use crate::games::{BotType, GameBroadcaster, GameSessionConfig, SessionRng};
use crate::replay::ReplayRecorder;
use super::bot_controller::BotController;
use super::game_state::SnakeGameState;
use super::settings::SnakeSessionSettings;
use super::types::{DeadSnakeBehavior, DeathReason, Direction, FieldSize, Point, WallCollisionMode};

#[derive(Clone)]
pub struct SnakeSessionState {
    pub session_id: String,
    pub game_state: Arc<Mutex<SnakeGameState>>,
    pub tick: Arc<Mutex<u64>>,
    pub rng: Arc<Mutex<SessionRng>>,
    pub bots: HashMap<BotId, BotType>,
    pub tick_interval: Duration,
    pub replay_recorder: Option<Arc<Mutex<ReplayRecorder>>>,
}

impl SnakeSessionState {
    pub fn create(
        config: &GameSessionConfig,
        settings: &SnakeSessionSettings,
        seed: u64,
        replay_recorder: Option<Arc<Mutex<ReplayRecorder>>>,
    ) -> Self {
        let rng = SessionRng::new(seed);

        let field_size = FieldSize {
            width: settings.field_width,
            height: settings.field_height,
        };
        let mut game_state = SnakeGameState::new(
            field_size,
            settings.wall_collision_mode,
            settings.dead_snake_behavior,
            settings.max_food_count,
            settings.food_spawn_probability,
        );

        let total_players = config.human_players.len() + config.bots.len();
        let mut idx = 0;

        for player_id in &config.human_players {
            let start_pos =
                calculate_start_position(idx, total_players, settings.field_width, settings.field_height);
            let direction = calculate_start_direction(idx, total_players);
            game_state.add_snake(player_id.clone(), start_pos, direction);
            idx += 1;
        }

        for bot_id in config.bots.keys() {
            let start_pos =
                calculate_start_position(idx, total_players, settings.field_width, settings.field_height);
            let direction = calculate_start_direction(idx, total_players);
            game_state.add_snake(bot_id.to_player_id(), start_pos, direction);
            idx += 1;
        }

        Self {
            session_id: config.session_id.clone(),
            game_state: Arc::new(Mutex::new(game_state)),
            tick: Arc::new(Mutex::new(0u64)),
            rng: Arc::new(Mutex::new(rng)),
            bots: config.bots.clone(),
            tick_interval: settings.tick_interval,
            replay_recorder,
        }
    }
}

pub struct SnakeSession;

impl SnakeSession {
    pub async fn run(
        config: GameSessionConfig,
        session_state: SnakeSessionState,
        broadcaster: impl GameBroadcaster,
    ) -> GameOverNotification {
        let initial_player_count = config.human_players.len() + config.bots.len();
        let mut tick_interval_timer = interval(session_state.tick_interval);

        loop {
            tick_interval_timer.tick().await;

            let current_tick = {
                let tick = session_state.tick.lock().await;
                *tick
            };

            let mut game_state = session_state.game_state.lock().await;
            let mut rng = session_state.rng.lock().await;

            for (bot_id, bot_type) in &session_state.bots {
                if let BotType::Snake(snake_bot_type) = bot_type {
                    let player_id = bot_id.to_player_id();
                    if let Some(direction) =
                        BotController::calculate_move(*snake_bot_type, &player_id, &game_state, &mut rng)
                    {
                        if let Err(e) = game_state.set_snake_direction(&player_id, direction) {
                            log!("[session:{}] Bot {} failed to set direction: {}", session_state.session_id, player_id, e);
                        }

                        if let Some(ref recorder) = session_state.replay_recorder {
                            let mut recorder = recorder.lock().await;
                            if let Some(player_index) = recorder.find_player_index(&player_id.to_string())
                            {
                                let command = create_turn_command(direction);
                                recorder.record_command(current_tick as i64, player_index, command);
                            }
                        }
                    }
                }
            }

            game_state.update(&mut rng);
            drop(rng);

            let mut tick_value = session_state.tick.lock().await;
            *tick_value += 1;

            let proto_state =
                build_proto_state(&game_state, &session_state.bots, *tick_value, session_state.tick_interval);
            drop(tick_value);

            let recipients = config.get_all_recipients();
            let state_update = GameStateUpdate {
                state: Some(game_state_update::State::Snake(proto_state)),
            };
            broadcaster.broadcast_state(state_update, recipients).await;

            let alive_count = game_state.snakes.values().filter(|s| s.is_alive()).count();
            let game_over = if initial_player_count == 1 {
                alive_count == 0
            } else {
                alive_count <= 1
            };

            drop(game_state);

            if game_over {
                break;
            }
        }

        build_game_over_notification(&session_state).await
    }

    pub async fn handle_command(
        state: &SnakeSessionState,
        client_id: &ClientId,
        command: &SnakeInGameCommand,
    ) {
        let direction = match &command.command {
            Some(snake_in_game_command::Command::Turn(turn_cmd)) => {
                match ProtoDirection::try_from(turn_cmd.direction) {
                    Ok(ProtoDirection::Up) => Direction::Up,
                    Ok(ProtoDirection::Down) => Direction::Down,
                    Ok(ProtoDirection::Left) => Direction::Left,
                    Ok(ProtoDirection::Right) => Direction::Right,
                    _ => return,
                }
            }
            _ => return,
        };

        if let Some(ref recorder) = state.replay_recorder {
            let current_tick = *state.tick.lock().await;
            let mut recorder = recorder.lock().await;
            if let Some(player_index) = recorder.find_player_index(&client_id.to_string()) {
                let in_game_command = create_turn_command(direction);
                recorder.record_command(current_tick as i64, player_index, in_game_command);
            }
        }

        let mut state_guard = state.game_state.lock().await;
        let player_id = PlayerId::new(client_id.to_string());
        if let Err(e) = state_guard.set_snake_direction(&player_id, direction) {
            log!("[session:{}] Player {} failed to set direction: {}", state.session_id, player_id, e);
        }
    }

    pub async fn handle_kill_snake(
        state: &SnakeSessionState,
        client_id: &ClientId,
        reason: DeathReason,
    ) {
        let mut state_guard = state.game_state.lock().await;
        let player_id = PlayerId::new(client_id.to_string());
        if let Err(e) = state_guard.kill_snake(&player_id, reason) {
            log!("[session:{}] Failed to kill snake {}: {}", state.session_id, player_id, e);
        }
    }
}

fn calculate_start_position(index: usize, total: usize, width: usize, height: usize) -> Point {
    let spacing = if total <= 2 {
        width / (total + 1)
    } else {
        width / total
    };

    let x = if total == 1 {
        width / 2
    } else {
        (index + 1) * spacing
    };

    let y = height / 2;

    Point::new(x.min(width - 1), y)
}

fn calculate_start_direction(_index: usize, _total: usize) -> Direction {
    Direction::Up
}

fn build_proto_state(
    state: &SnakeGameState,
    bots: &HashMap<BotId, BotType>,
    tick_value: u64,
    tick_interval: Duration,
) -> ProtoSnakeGameState {
    let mut snakes = vec![];

    for (id, snake) in &state.snakes {
        let segments = snake
            .body
            .iter()
            .map(|p| SnakePosition {
                x: p.x as i32,
                y: p.y as i32,
            })
            .collect();

        let is_bot = bots.iter().any(|(bot_id, _)| bot_id.to_player_id() == *id);

        snakes.push(crate::proto::snake::Snake {
            identity: Some(crate::proto::snake::PlayerIdentity {
                player_id: id.to_string(),
                is_bot,
            }),
            segments,
            alive: snake.is_alive(),
            score: snake.score,
        });
    }

    let food: Vec<SnakePosition> = state
        .food_set
        .iter()
        .map(|p| SnakePosition {
            x: p.x as i32,
            y: p.y as i32,
        })
        .collect();

    let dead_snake_behavior_proto = match state.dead_snake_behavior {
        DeadSnakeBehavior::Disappear => crate::proto::snake::DeadSnakeBehavior::Disappear,
        DeadSnakeBehavior::StayOnField => crate::proto::snake::DeadSnakeBehavior::StayOnField,
    };

    let wall_collision_mode_proto = match state.wall_collision_mode {
        WallCollisionMode::Death => crate::proto::snake::WallCollisionMode::Death,
        WallCollisionMode::WrapAround => crate::proto::snake::WallCollisionMode::WrapAround,
    };

    ProtoSnakeGameState {
        tick: tick_value,
        snakes,
        food,
        field_width: state.field_size.width as u32,
        field_height: state.field_size.height as u32,
        tick_interval_ms: tick_interval.as_millis() as u32,
        wall_collision_mode: wall_collision_mode_proto as i32,
        dead_snake_behavior: dead_snake_behavior_proto as i32,
    }
}

async fn build_game_over_notification(session_state: &SnakeSessionState) -> GameOverNotification {
    let game_state = session_state.game_state.lock().await;

    let scores: Vec<ScoreEntry> = game_state
        .snakes
        .iter()
        .map(|(id, snake)| {
            let is_bot = session_state
                .bots
                .iter()
                .any(|(bot_id, _)| bot_id.to_player_id() == *id);

            ScoreEntry {
                identity: Some(PlayerIdentity {
                    player_id: id.to_string(),
                    is_bot,
                }),
                score: snake.score,
            }
        })
        .collect();

    let winner = game_state
        .snakes
        .iter()
        .find(|(_, snake)| snake.is_alive())
        .map(|(id, _)| {
            let is_bot = session_state
                .bots
                .iter()
                .any(|(bot_id, _)| bot_id.to_player_id() == *id);
            PlayerIdentity {
                player_id: id.to_string(),
                is_bot,
            }
        });

    let game_end_reason = game_state
        .game_end_reason
        .map(|r| match r {
            DeathReason::WallCollision => SnakeGameEndReason::WallCollision,
            DeathReason::SelfCollision => SnakeGameEndReason::SelfCollision,
            DeathReason::OtherSnakeCollision => SnakeGameEndReason::SnakeCollision,
            DeathReason::PlayerDisconnected => SnakeGameEndReason::PlayerDisconnected,
        })
        .unwrap_or(SnakeGameEndReason::GameCompleted);

    GameOverNotification {
        scores,
        winner,
        game_info: Some(game_over_notification::GameInfo::SnakeInfo(
            SnakeGameEndInfo {
                reason: game_end_reason as i32,
            },
        )),
    }
}

fn create_turn_command(direction: Direction) -> InGameCommand {
    let proto_direction = match direction {
        Direction::Up => ProtoDirection::Up,
        Direction::Down => ProtoDirection::Down,
        Direction::Left => ProtoDirection::Left,
        Direction::Right => ProtoDirection::Right,
    };

    InGameCommand {
        command: Some(in_game_command::Command::Snake(SnakeInGameCommand {
            command: Some(snake_in_game_command::Command::Turn(TurnCommand {
                direction: proto_direction as i32,
            })),
        })),
    }
}
