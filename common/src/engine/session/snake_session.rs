use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;

use crate::{
    BotId,
    GameStateUpdate, game_state_update, GameOverNotification, game_over_notification,
    ScoreEntry, PlayerIdentity, SnakePosition,
    proto::snake::{SnakeGameState as ProtoSnakeGameState, SnakeGameEndReason, SnakeGameEndInfo},
};
use crate::lobby::BotType;
use crate::engine::snake::{GameState, Direction, Point, DeathReason, BotController, FieldSize, WallCollisionMode, DeadSnakeBehavior};
use crate::engine::session::{GameBroadcaster, GameSessionConfig};

pub struct SnakeSessionState {
    pub game_state: Arc<Mutex<GameState>>,
    pub tick: Arc<Mutex<u64>>,
    pub bots: HashMap<BotId, BotType>,
    pub tick_interval: Duration,
}

pub struct SnakeSessionSettings {
    pub field_width: usize,
    pub field_height: usize,
    pub wall_collision_mode: WallCollisionMode,
    pub dead_snake_behavior: DeadSnakeBehavior,
    pub max_food_count: usize,
    pub food_spawn_probability: f32,
    pub tick_interval: Duration,
}

impl From<&crate::proto::snake::SnakeLobbySettings> for SnakeSessionSettings {
    fn from(settings: &crate::proto::snake::SnakeLobbySettings) -> Self {
        let wall_collision_mode = match crate::proto::snake::WallCollisionMode::try_from(settings.wall_collision_mode) {
            Ok(crate::proto::snake::WallCollisionMode::Death) |
            Ok(crate::proto::snake::WallCollisionMode::Unspecified) => WallCollisionMode::Death,
            Ok(crate::proto::snake::WallCollisionMode::WrapAround) => WallCollisionMode::WrapAround,
            _ => WallCollisionMode::Death,
        };
        let dead_snake_behavior = match crate::proto::snake::DeadSnakeBehavior::try_from(settings.dead_snake_behavior) {
            Ok(crate::proto::snake::DeadSnakeBehavior::StayOnField) => DeadSnakeBehavior::StayOnField,
            Ok(crate::proto::snake::DeadSnakeBehavior::Disappear | crate::proto::snake::DeadSnakeBehavior::Unspecified) |
            Err(_) => DeadSnakeBehavior::Disappear,
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

pub fn create_session(
    config: &GameSessionConfig,
    settings: &SnakeSessionSettings,
) -> SnakeSessionState {
    let field_size = FieldSize {
        width: settings.field_width,
        height: settings.field_height,
    };
    let mut game_state = GameState::new(
        field_size,
        settings.wall_collision_mode,
        settings.dead_snake_behavior,
        settings.max_food_count,
        settings.food_spawn_probability,
    );

    let total_players = config.human_players.len() + config.bots.len();
    let mut idx = 0;

    for player_id in &config.human_players {
        let start_pos = calculate_start_position(idx, total_players, settings.field_width, settings.field_height);
        let direction = calculate_start_direction(idx, total_players);
        game_state.add_snake(player_id.clone(), start_pos, direction);
        idx += 1;
    }

    for bot_id in config.bots.keys() {
        let start_pos = calculate_start_position(idx, total_players, settings.field_width, settings.field_height);
        let direction = calculate_start_direction(idx, total_players);
        game_state.add_snake(bot_id.to_player_id(), start_pos, direction);
        idx += 1;
    }

    SnakeSessionState {
        game_state: Arc::new(Mutex::new(game_state)),
        tick: Arc::new(Mutex::new(0u64)),
        bots: config.bots.clone(),
        tick_interval: settings.tick_interval,
    }
}

pub async fn run_game_loop<B: GameBroadcaster>(
    config: GameSessionConfig,
    session_state: SnakeSessionState,
    broadcaster: B,
) -> GameOverNotification {
    let initial_player_count = config.human_players.len() + config.bots.len();
    let mut tick_interval_timer = interval(session_state.tick_interval);

    loop {
        tick_interval_timer.tick().await;

        let mut game_state = session_state.game_state.lock().await;

        for (bot_id, bot_type) in &session_state.bots {
            if let BotType::Snake(snake_bot_type) = bot_type {
                let player_id = bot_id.to_player_id();
                if let Some(direction) = BotController::calculate_move(*snake_bot_type, &player_id, &game_state) {
                    game_state.set_snake_direction(&player_id, direction);
                }
            }
        }

        game_state.update();

        let mut tick_value = session_state.tick.lock().await;
        *tick_value += 1;

        let proto_state = build_proto_state(&game_state, &session_state.bots, *tick_value, session_state.tick_interval);
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

    build_game_over_notification(&config, &session_state).await
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
    state: &GameState,
    bots: &HashMap<BotId, BotType>,
    tick_value: u64,
    tick_interval: Duration,
) -> ProtoSnakeGameState {
    let mut snakes = vec![];

    for (id, snake) in &state.snakes {
        let segments = snake.body.iter().map(|p| SnakePosition {
            x: p.x as i32,
            y: p.y as i32,
        }).collect();

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

    let food: Vec<SnakePosition> = state.food_set.iter().map(|p| SnakePosition {
        x: p.x as i32,
        y: p.y as i32,
    }).collect();

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

async fn build_game_over_notification(
    _config: &GameSessionConfig,
    session_state: &SnakeSessionState,
) -> GameOverNotification {
    let game_state = session_state.game_state.lock().await;

    let scores: Vec<ScoreEntry> = game_state.snakes.iter().map(|(id, snake)| {
        let is_bot = session_state.bots.iter().any(|(bot_id, _)| bot_id.to_player_id() == *id);

        ScoreEntry {
            identity: Some(PlayerIdentity {
                player_id: id.to_string(),
                is_bot,
            }),
            score: snake.score,
        }
    }).collect();

    let winner = game_state.snakes.iter()
        .find(|(_, snake)| snake.is_alive())
        .map(|(id, _)| {
            let is_bot = session_state.bots.iter().any(|(bot_id, _)| bot_id.to_player_id() == *id);
            PlayerIdentity {
                player_id: id.to_string(),
                is_bot,
            }
        });

    let game_end_reason = game_state.game_end_reason
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
            }
        )),
    }
}
