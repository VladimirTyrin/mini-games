use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;
use common::{ClientId, PlayerId, BotId, log, ServerMessage, server_message, SnakePosition};
use crate::lobby_manager::BotType;
use crate::games::{SharedContext, GameSessionResult, GameOverResult, GameStateEnum};
use super::{GameState, FieldSize, WallCollisionMode, DeadSnakeBehavior, Direction, Point, DeathReason, BotController};

pub fn create_session(
    ctx: &SharedContext,
    settings: &common::proto::snake::SnakeLobbySettings,
) -> GameSessionResult {
    let field_width = settings.field_width as usize;
    let field_height = settings.field_height as usize;
    let wall_collision_mode = match common::proto::snake::WallCollisionMode::try_from(settings.wall_collision_mode) {
        Ok(common::proto::snake::WallCollisionMode::Death) |
        Ok(common::proto::snake::WallCollisionMode::Unspecified) => WallCollisionMode::Death,
        Ok(common::proto::snake::WallCollisionMode::WrapAround) => WallCollisionMode::WrapAround,
        _ => WallCollisionMode::Death,
    };
    let dead_snake_behavior = match common::proto::snake::DeadSnakeBehavior::try_from(settings.dead_snake_behavior) {
        Ok(common::proto::snake::DeadSnakeBehavior::StayOnField) => DeadSnakeBehavior::StayOnField,
        Ok(common::proto::snake::DeadSnakeBehavior::Disappear | common::proto::snake::DeadSnakeBehavior::Unspecified) |
        Err(_) => DeadSnakeBehavior::Disappear,
    };
    let max_food_count = settings.max_food_count.max(1) as usize;
    let food_spawn_probability = settings.food_spawn_probability.clamp(0.001, 1.0);

    let field_size = FieldSize {
        width: field_width,
        height: field_height,
    };
    let mut game_state = GameState::new(field_size, wall_collision_mode, dead_snake_behavior, max_food_count, food_spawn_probability);

    let total_players = ctx.human_players.len() + ctx.bots.len();
    let mut idx = 0;

    for player_id in &ctx.human_players {
        let start_pos = calculate_start_position(idx, total_players, field_width, field_height);
        let direction = calculate_start_direction(idx, total_players);
        game_state.add_snake(player_id.clone(), start_pos, direction);
        idx += 1;
    }

    for bot_id in ctx.bots.keys() {
        let start_pos = calculate_start_position(idx, total_players, field_width, field_height);
        let direction = calculate_start_direction(idx, total_players);
        game_state.add_snake(bot_id.to_player_id(), start_pos, direction);
        idx += 1;
    }

    GameSessionResult {
        state: Arc::new(Mutex::new(GameStateEnum::Snake(game_state))),
        tick: Arc::new(Mutex::new(0u64)),
        bots: Arc::new(Mutex::new(ctx.bots.clone())),
        observers: Arc::new(Mutex::new(ctx.observers.clone())),
    }
}

pub async fn run_game_loop(
    ctx: SharedContext,
    state: Arc<Mutex<GameStateEnum>>,
    tick: Arc<Mutex<u64>>,
    bots: Arc<Mutex<HashMap<BotId, BotType>>>,
    observers: Arc<Mutex<HashSet<PlayerId>>>,
    tick_interval: Duration,
) -> GameOverResult {
    let initial_player_count = ctx.human_players.len() + ctx.bots.len();
    let mut tick_interval_timer = interval(tick_interval);

    loop {
        tick_interval_timer.tick().await;

        let mut state_guard = state.lock().await;
        let game_state = match &mut *state_guard {
            GameStateEnum::Snake(s) => s,
            _ => {
                log!("Invalid game state type in Snake game loop");
                break;
            }
        };

        let bots_map = bots.lock().await;
        for (bot_id, bot_type) in bots_map.iter() {
            if let BotType::Snake(snake_bot_type) = bot_type {
                let player_id = bot_id.to_player_id();
                if let Some(direction) = BotController::calculate_move(*snake_bot_type, &player_id, game_state) {
                    game_state.set_snake_direction(&player_id, direction);
                }
            }
        }
        drop(bots_map);

        game_state.update();

        let mut tick_value = tick.lock().await;
        *tick_value += 1;

        let proto_state = build_proto_state(game_state, &bots, *tick_value, tick_interval).await;
        drop(tick_value);

        let client_ids = build_client_ids(&ctx.human_players, &observers).await;
        let game_state_msg = ServerMessage {
            message: Some(server_message::Message::GameState(
                common::GameStateUpdate {
                    state: Some(common::game_state_update::State::Snake(proto_state))
                }
            )),
        };
        ctx.broadcaster.broadcast_to_clients(&client_ids, game_state_msg).await;

        let alive_count = game_state.snakes.values().filter(|s| s.is_alive()).count();
        let game_over = if initial_player_count == 1 {
            alive_count == 0
        } else {
            alive_count <= 1
        };

        drop(state_guard);

        if game_over {
            break;
        }
    }

    build_game_over_result(&ctx, &state, &bots, &observers).await
}

pub async fn handle_direction(
    state: &Arc<Mutex<GameStateEnum>>,
    client_id: &ClientId,
    direction: Direction,
) {
    let mut state_guard = state.lock().await;
    if let GameStateEnum::Snake(game_state) = &mut *state_guard {
        let player_id = PlayerId::new(client_id.to_string());
        game_state.set_snake_direction(&player_id, direction);
    }
}

pub async fn handle_kill_snake(
    state: &Arc<Mutex<GameStateEnum>>,
    client_id: &ClientId,
    reason: DeathReason,
) {
    let mut state_guard = state.lock().await;
    if let GameStateEnum::Snake(game_state) = &mut *state_guard {
        let player_id = PlayerId::new(client_id.to_string());
        game_state.kill_snake(&player_id, reason);
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

async fn build_proto_state(
    state: &GameState,
    bots: &Arc<Mutex<HashMap<BotId, BotType>>>,
    tick_value: u64,
    tick_interval: Duration,
) -> common::proto::snake::SnakeGameState {
    let bots_ref = bots.lock().await;
    let mut snakes = vec![];

    for (id, snake) in &state.snakes {
        let segments = snake.body.iter().map(|p| SnakePosition {
            x: p.x as i32,
            y: p.y as i32,
        }).collect();

        let is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == *id);

        snakes.push(common::proto::snake::Snake {
            identity: Some(common::proto::snake::PlayerIdentity {
                player_id: id.to_string(),
                is_bot,
            }),
            segments,
            alive: snake.is_alive(),
            score: snake.score,
        });
    }
    drop(bots_ref);

    let food: Vec<SnakePosition> = state.food_set.iter().map(|p| SnakePosition {
        x: p.x as i32,
        y: p.y as i32,
    }).collect();

    let dead_snake_behavior_proto = match state.dead_snake_behavior {
        DeadSnakeBehavior::Disappear => common::proto::snake::DeadSnakeBehavior::Disappear,
        DeadSnakeBehavior::StayOnField => common::proto::snake::DeadSnakeBehavior::StayOnField,
    };

    let wall_collision_mode_proto = match state.wall_collision_mode {
        WallCollisionMode::Death => common::proto::snake::WallCollisionMode::Death,
        WallCollisionMode::WrapAround => common::proto::snake::WallCollisionMode::WrapAround,
    };

    common::proto::snake::SnakeGameState {
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

async fn build_client_ids(
    human_players: &[PlayerId],
    observers: &Arc<Mutex<HashSet<PlayerId>>>,
) -> Vec<ClientId> {
    let observers_set = observers.lock().await;
    let mut client_ids: Vec<ClientId> = human_players.iter()
        .map(|p| ClientId::new(p.to_string()))
        .collect();
    client_ids.extend(observers_set.iter().map(|p| ClientId::new(p.to_string())));
    client_ids
}

async fn build_game_over_result(
    ctx: &SharedContext,
    state: &Arc<Mutex<GameStateEnum>>,
    bots: &Arc<Mutex<HashMap<BotId, BotType>>>,
    observers: &Arc<Mutex<HashSet<PlayerId>>>,
) -> GameOverResult {
    let state_guard = state.lock().await;
    let game_state = match &*state_guard {
        GameStateEnum::Snake(s) => s,
        _ => panic!("Invalid game state type in game over handling"),
    };

    let bots_ref = bots.lock().await;
    let scores: Vec<common::ScoreEntry> = game_state.snakes.iter().map(|(id, snake)| {
        let is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == *id);

        common::ScoreEntry {
            identity: Some(common::PlayerIdentity {
                player_id: id.to_string(),
                is_bot,
            }),
            score: snake.score,
        }
    }).collect();

    let winner = game_state.snakes.iter()
        .find(|(_, snake)| snake.is_alive())
        .map(|(id, _)| {
            let is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == *id);
            common::PlayerIdentity {
                player_id: id.to_string(),
                is_bot,
            }
        });
    drop(bots_ref);

    let game_end_reason = game_state.game_end_reason
        .map(|r| match r {
            DeathReason::WallCollision => common::proto::snake::SnakeGameEndReason::WallCollision,
            DeathReason::SelfCollision => common::proto::snake::SnakeGameEndReason::SelfCollision,
            DeathReason::OtherSnakeCollision => common::proto::snake::SnakeGameEndReason::SnakeCollision,
            DeathReason::PlayerDisconnected => common::proto::snake::SnakeGameEndReason::PlayerDisconnected,
        })
        .unwrap_or(common::proto::snake::SnakeGameEndReason::GameCompleted);

    let observers_set = observers.lock().await;
    let current_observers = observers_set.clone();
    drop(observers_set);

    GameOverResult {
        session_id: ctx.session_id.clone(),
        scores,
        winner,
        game_info: common::game_over_notification::GameInfo::SnakeInfo(
            common::proto::snake::SnakeGameEndInfo {
                reason: game_end_reason as i32,
            }
        ),
        human_players: ctx.human_players.clone(),
        observers: current_observers,
    }
}
