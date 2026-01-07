use std::path::PathBuf;
use std::time::Duration;
use std::collections::HashMap;
use tokio::sync::mpsc;

use common::engine::snake::{GameState as SnakeGameState, Direction, FieldSize, WallCollisionMode, DeadSnakeBehavior, Point, DeathReason};
use common::engine::tictactoe::{TicTacToeGameState, FirstPlayerMode};
use common::engine::session::SessionRng;
use common::replay::{load_replay, ReplayPlayer};
use common::{
    ReplayGame, GameStateUpdate, game_state_update, PlayerAction, player_action_content,
    in_game_command, lobby_settings, SnakePosition,
    proto::snake::SnakeGameState as ProtoSnakeGameState,
    PlayerId,
};
use common::version::VERSION;
use crate::state::{AppState, SharedState};

pub enum ReplayCommand {
    Pause,
    Resume,
    Stop,
    SetSpeed(f32),
}

pub async fn run_replay_playback(
    file_path: PathBuf,
    shared_state: SharedState,
    mut command_rx: mpsc::UnboundedReceiver<ReplayCommand>,
) {
    let replay = match load_replay(&file_path) {
        Ok(r) => r,
        Err(e) => {
            shared_state.set_error(format!("Failed to load replay: {}", e));
            return;
        }
    };

    let player = ReplayPlayer::new(replay);
    let replay_version = player.engine_version().to_string();
    if replay_version != VERSION {
        common::log!("Warning: Replay version {} differs from client version {}", replay_version, VERSION);
    }

    let game = player.game();

    match game {
        ReplayGame::Snake => {
            run_snake_replay(player, shared_state, &mut command_rx, replay_version).await;
        }
        ReplayGame::Tictactoe => {
            run_tictactoe_replay(player, shared_state, &mut command_rx, replay_version).await;
        }
        ReplayGame::Unspecified => {
            shared_state.set_error("Unknown game type in replay".to_string());
        }
    }
}

async fn run_snake_replay(
    mut player: ReplayPlayer,
    shared_state: SharedState,
    command_rx: &mut mpsc::UnboundedReceiver<ReplayCommand>,
    replay_version: String,
) {
    let settings = match player.lobby_settings() {
        Some(lobby_settings::Settings::Snake(s)) => s.clone(),
        _ => {
            shared_state.set_error("Invalid snake settings in replay".to_string());
            return;
        }
    };

    let wall_collision_mode = match common::proto::snake::WallCollisionMode::try_from(settings.wall_collision_mode) {
        Ok(common::proto::snake::WallCollisionMode::WrapAround) => WallCollisionMode::WrapAround,
        _ => WallCollisionMode::Death,
    };

    let dead_snake_behavior = match common::proto::snake::DeadSnakeBehavior::try_from(settings.dead_snake_behavior) {
        Ok(common::proto::snake::DeadSnakeBehavior::StayOnField) => DeadSnakeBehavior::StayOnField,
        _ => DeadSnakeBehavior::Disappear,
    };

    let field_width = settings.field_width as usize;
    let field_height = settings.field_height as usize;
    let field_size = FieldSize {
        width: field_width,
        height: field_height,
    };

    let mut game_state = SnakeGameState::new(
        field_size,
        wall_collision_mode,
        dead_snake_behavior,
        settings.max_food_count.max(1) as usize,
        settings.food_spawn_probability.clamp(0.001, 1.0),
    );

    let players = player.players();
    let total_players = players.len();
    let player_map: HashMap<i32, PlayerId> = players.iter()
        .enumerate()
        .map(|(i, p)| (i as i32, PlayerId::new(p.player_id.clone())))
        .collect();

    for (idx, p) in players.iter().enumerate() {
        let start_pos = calculate_snake_start_position(idx, total_players, field_width, field_height);
        game_state.add_snake(PlayerId::new(p.player_id.clone()), start_pos, Direction::Up);
    }

    let mut rng = SessionRng::new(player.seed());
    let tick_interval = Duration::from_millis(settings.tick_interval_ms as u64);
    let mut current_tick: u64 = 0;
    let mut is_paused = false;

    let total_ticks = estimate_total_ticks(&player);

    update_watching_state(&shared_state, &game_state, &players, is_paused, current_tick, total_ticks, &replay_version);

    let mut tick_timer = tokio::time::interval(tick_interval);

    loop {
        tokio::select! {
            _ = tick_timer.tick() => {
                if is_paused {
                    continue;
                }

                let actions = player.actions_for_tick(current_tick as i64);
                for action in actions {
                    apply_snake_action(&mut game_state, &action, &player_map);
                }

                game_state.update(&mut rng);
                current_tick += 1;

                update_watching_state(&shared_state, &game_state, &player.players(), is_paused, current_tick.min(total_ticks), total_ticks, &replay_version);

                let alive_count = game_state.snakes.values().filter(|s| s.is_alive()).count();
                let game_over = if total_players == 1 {
                    alive_count == 0
                } else {
                    alive_count <= 1
                };

                if game_over || player.is_finished() {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    break;
                }
            }
            Some(cmd) = command_rx.recv() => {
                match cmd {
                    ReplayCommand::Pause => {
                        is_paused = true;
                        update_watching_state(&shared_state, &game_state, &player.players(), is_paused, current_tick, total_ticks, &replay_version);
                    }
                    ReplayCommand::Resume => {
                        is_paused = false;
                        update_watching_state(&shared_state, &game_state, &player.players(), is_paused, current_tick, total_ticks, &replay_version);
                    }
                    ReplayCommand::Stop => {
                        break;
                    }
                    ReplayCommand::SetSpeed(speed) => {
                        let clamped_speed = speed.clamp(0.25, 4.0);
                        let adjusted_interval = Duration::from_millis(
                            (settings.tick_interval_ms as f32 / clamped_speed) as u64
                        );
                        tick_timer = tokio::time::interval(adjusted_interval);
                    }
                }
            }
        }
    }
}

async fn run_tictactoe_replay(
    mut player: ReplayPlayer,
    shared_state: SharedState,
    command_rx: &mut mpsc::UnboundedReceiver<ReplayCommand>,
    replay_version: String,
) {
    let settings = match player.lobby_settings() {
        Some(lobby_settings::Settings::Tictactoe(s)) => s.clone(),
        _ => {
            shared_state.set_error("Invalid tictactoe settings in replay".to_string());
            return;
        }
    };

    let players = player.players();
    if players.len() != 2 {
        shared_state.set_error("TicTacToe replay must have exactly 2 players".to_string());
        return;
    }

    let player_ids: Vec<PlayerId> = players.iter()
        .map(|p| PlayerId::new(p.player_id.clone()))
        .collect();

    let player_map: HashMap<i32, PlayerId> = players.iter()
        .enumerate()
        .map(|(i, p)| (i as i32, PlayerId::new(p.player_id.clone())))
        .collect();

    let mut rng = SessionRng::new(player.seed());
    let mut game_state = TicTacToeGameState::new(
        settings.field_width as usize,
        settings.field_height as usize,
        settings.win_count as usize,
        player_ids,
        FirstPlayerMode::Random,
        &mut rng,
    );

    let total_actions = player.total_actions() as u64;
    let mut current_action: u64 = 0;
    let mut is_paused = false;

    update_tictactoe_watching_state(&shared_state, &game_state, &players, is_paused, current_action, total_actions, &replay_version);

    let move_delay = Duration::from_millis(1000);
    let mut move_timer = tokio::time::interval(move_delay);

    loop {
        tokio::select! {
            _ = move_timer.tick() => {
                if is_paused {
                    continue;
                }

                if let Some(action) = player.next_action() {
                    apply_tictactoe_action(&mut game_state, action, &player_map);
                    current_action += 1;
                    update_tictactoe_watching_state(&shared_state, &game_state, &player.players(), is_paused, current_action.min(total_actions), total_actions, &replay_version);
                }

                if player.is_finished() || game_state.status != common::engine::tictactoe::GameStatus::InProgress {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    break;
                }
            }
            Some(cmd) = command_rx.recv() => {
                match cmd {
                    ReplayCommand::Pause => {
                        is_paused = true;
                        update_tictactoe_watching_state(&shared_state, &game_state, &player.players(), is_paused, current_action, total_actions, &replay_version);
                    }
                    ReplayCommand::Resume => {
                        is_paused = false;
                        update_tictactoe_watching_state(&shared_state, &game_state, &player.players(), is_paused, current_action, total_actions, &replay_version);
                    }
                    ReplayCommand::Stop => {
                        break;
                    }
                    ReplayCommand::SetSpeed(speed) => {
                        let adjusted_delay = Duration::from_millis(
                            (1000.0 / speed.clamp(0.25, 4.0)) as u64
                        );
                        move_timer = tokio::time::interval(adjusted_delay);
                    }
                }
            }
        }
    }
}

fn apply_snake_action(
    game_state: &mut SnakeGameState,
    action: &PlayerAction,
    player_map: &HashMap<i32, PlayerId>,
) {
    let Some(player_id) = player_map.get(&action.player_index) else {
        return;
    };

    let Some(content) = &action.content else {
        return;
    };

    let Some(inner) = &content.content else {
        return;
    };

    match inner {
        player_action_content::Content::Command(cmd) => {
            if let Some(in_game_command::Command::Snake(snake_cmd)) = &cmd.command {
                if let Some(common::proto::snake::snake_in_game_command::Command::Turn(turn)) = &snake_cmd.command {
                    let direction = match common::proto::snake::Direction::try_from(turn.direction) {
                        Ok(common::proto::snake::Direction::Up) => Direction::Up,
                        Ok(common::proto::snake::Direction::Down) => Direction::Down,
                        Ok(common::proto::snake::Direction::Left) => Direction::Left,
                        Ok(common::proto::snake::Direction::Right) => Direction::Right,
                        _ => return,
                    };
                    game_state.set_snake_direction(player_id, direction);
                }
            }
        }
        player_action_content::Content::Disconnected(_) => {
            game_state.kill_snake(player_id, DeathReason::PlayerDisconnected);
        }
    }
}

fn apply_tictactoe_action(
    game_state: &mut TicTacToeGameState,
    action: &PlayerAction,
    player_map: &HashMap<i32, PlayerId>,
) {
    let Some(player_id) = player_map.get(&action.player_index) else {
        return;
    };

    let Some(content) = &action.content else {
        return;
    };

    let Some(inner) = &content.content else {
        return;
    };

    if let player_action_content::Content::Command(cmd) = inner {
        if let Some(in_game_command::Command::Tictactoe(ttt_cmd)) = &cmd.command {
            if let Some(common::proto::tictactoe::tic_tac_toe_in_game_command::Command::Place(place)) = &ttt_cmd.command {
                let _ = game_state.place_mark(player_id, place.x as usize, place.y as usize);
            }
        }
    }
}

fn calculate_snake_start_position(index: usize, total: usize, width: usize, height: usize) -> Point {
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

    Point::new(x.min(width - 1), height / 2)
}

fn estimate_total_ticks(player: &ReplayPlayer) -> u64 {
    let total = player.total_actions();
    if total == 0 {
        return 1;
    }

    let mut temp_player = ReplayPlayer::new(player.replay_ref().clone());
    let mut max_tick: i64 = 0;

    while let Some(action) = temp_player.next_action() {
        if action.tick > max_tick {
            max_tick = action.tick;
        }
    }

    max_tick.max(1) as u64
}

fn update_watching_state(
    shared_state: &SharedState,
    game_state: &SnakeGameState,
    players: &[common::PlayerIdentity],
    is_paused: bool,
    current_tick: u64,
    total_ticks: u64,
    replay_version: &str,
) {
    let proto_state = build_snake_proto_state(game_state, players, current_tick);
    let state_update = GameStateUpdate {
        state: Some(game_state_update::State::Snake(proto_state)),
    };

    shared_state.set_state(AppState::WatchingReplay {
        game_state: Some(state_update),
        is_paused,
        current_tick,
        total_ticks,
        replay_version: replay_version.to_string(),
    });
}

fn update_tictactoe_watching_state(
    shared_state: &SharedState,
    game_state: &TicTacToeGameState,
    players: &[common::PlayerIdentity],
    is_paused: bool,
    current_action: u64,
    total_actions: u64,
    replay_version: &str,
) {
    let player_x_is_bot = players.iter()
        .find(|p| p.player_id == game_state.player_x.to_string())
        .map(|p| p.is_bot)
        .unwrap_or(false);
    let player_o_is_bot = players.iter()
        .find(|p| p.player_id == game_state.player_o.to_string())
        .map(|p| p.is_bot)
        .unwrap_or(false);
    let current_player_is_bot = players.iter()
        .find(|p| p.player_id == game_state.current_player.to_string())
        .map(|p| p.is_bot)
        .unwrap_or(false);

    let proto_state = game_state.to_proto_state(player_x_is_bot, player_o_is_bot, current_player_is_bot);
    let state_update = GameStateUpdate {
        state: Some(game_state_update::State::Tictactoe(proto_state)),
    };

    shared_state.set_state(AppState::WatchingReplay {
        game_state: Some(state_update),
        is_paused,
        current_tick: current_action,
        total_ticks: total_actions,
        replay_version: replay_version.to_string(),
    });
}

fn build_snake_proto_state(
    state: &SnakeGameState,
    players: &[common::PlayerIdentity],
    tick: u64,
) -> ProtoSnakeGameState {
    let mut snakes = vec![];

    for (id, snake) in &state.snakes {
        let segments = snake.body.iter().map(|p| SnakePosition {
            x: p.x as i32,
            y: p.y as i32,
        }).collect();

        let is_bot = players.iter()
            .find(|p| p.player_id == id.to_string())
            .map(|p| p.is_bot)
            .unwrap_or(false);

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

    ProtoSnakeGameState {
        tick,
        snakes,
        food,
        field_width: state.field_size.width as u32,
        field_height: state.field_size.height as u32,
        tick_interval_ms: 100,
        wall_collision_mode: wall_collision_mode_proto as i32,
        dead_snake_behavior: dead_snake_behavior_proto as i32,
    }
}
