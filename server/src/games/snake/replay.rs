use std::collections::HashMap;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::games::snake::{
    DeadSnakeBehavior, DeathReason, Direction, FieldSize, Point, SnakeGameState, WallCollisionMode,
};
use crate::games::SessionRng;
use crate::replay::ReplayPlayer;
use crate::replay::session::{
    ReplayCommandResult, ReplaySessionCommand, broadcast_state_and_replay_info,
    estimate_total_ticks, handle_replay_command, wait_for_restart_or_stop,
};
use crate::{
    ClientId, GameStateUpdate, PlayerAction, PlayerId, SnakePosition, game_state_update,
    in_game_command, lobby_settings, player_action_content, log,
};
pub(crate) async fn run_replay(
    mut player: ReplayPlayer,
    command_rx: &mut mpsc::UnboundedReceiver<ReplaySessionCommand>,
    viewers: &[ClientId],
    host_only_control: bool,
    broadcaster: &crate::broadcaster::Broadcaster,
) -> bool {
    let settings = match player.lobby_settings() {
        Some(lobby_settings::Settings::Snake(s)) => *s,
        _ => return false,
    };

    let wall_collision_mode = match crate::proto::snake::WallCollisionMode::try_from(settings.wall_collision_mode) {
        Ok(crate::proto::snake::WallCollisionMode::WrapAround) => WallCollisionMode::WrapAround,
        _ => WallCollisionMode::Death,
    };

    let dead_snake_behavior = match crate::proto::snake::DeadSnakeBehavior::try_from(settings.dead_snake_behavior) {
        Ok(crate::proto::snake::DeadSnakeBehavior::StayOnField) => DeadSnakeBehavior::StayOnField,
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
    let player_map: HashMap<i32, PlayerId> = players
        .iter()
        .enumerate()
        .map(|(i, p)| (i as i32, PlayerId::new(p.player_id.clone())))
        .collect();

    for (idx, p) in players.iter().enumerate() {
        let start_pos = calculate_snake_start_position(idx, total_players, field_width, field_height);
        game_state.add_snake(PlayerId::new(p.player_id.clone()), start_pos, Direction::Up);
    }

    let mut rng = SessionRng::new(player.seed());
    let tick_interval_ms = settings.tick_interval_ms as f32;
    let mut current_tick: u64 = 0;
    let mut is_paused = false;
    let mut speed = 1.0_f32;

    let total_ticks = estimate_total_ticks(&player);

    let proto_state = build_snake_proto_state(&game_state, players, current_tick, settings.tick_interval_ms);
    let state_update = GameStateUpdate { state: Some(game_state_update::State::Snake(proto_state)) };
    broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_tick, total_ticks, speed, false, host_only_control).await;

    let mut tick_timer = tokio::time::interval(Duration::from_millis(tick_interval_ms as u64));

    loop {
        tokio::select! {
            _ = tick_timer.tick() => {
                if is_paused {
                    continue;
                }

                let actions = player.actions_for_tick(current_tick as i64);
                for action in &actions {
                    apply_snake_action(&mut game_state, action, &player_map);
                }

                game_state.update(&mut rng);
                current_tick += 1;

                let alive_count = game_state.snakes.values().filter(|s| s.is_alive()).count();
                let game_over = if total_players == 1 { alive_count == 0 } else { alive_count <= 1 };
                let is_finished = game_over || player.is_finished();

                let proto_state = build_snake_proto_state(&game_state, player.players(), current_tick.min(total_ticks), settings.tick_interval_ms);
                let state_update = GameStateUpdate { state: Some(game_state_update::State::Snake(proto_state)) };
                broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_tick.min(total_ticks), total_ticks, speed, is_finished, host_only_control).await;

                if is_finished {
                    return wait_for_restart_or_stop(command_rx).await;
                }
            }
            Some(cmd) = command_rx.recv() => {
                match cmd {
                    ReplaySessionCommand::ReplayCommand(replay_cmd) => {
                        match handle_replay_command(&replay_cmd, &mut is_paused, &mut speed) {
                            ReplayCommandResult::StateChanged => {
                                let proto_state = build_snake_proto_state(&game_state, player.players(), current_tick, settings.tick_interval_ms);
                                let state_update = GameStateUpdate { state: Some(game_state_update::State::Snake(proto_state)) };
                                broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_tick, total_ticks, speed, false, host_only_control).await;
                            }
                            ReplayCommandResult::SpeedChanged => {
                                let adjusted_interval = Duration::from_millis((tick_interval_ms / speed) as u64);
                                tick_timer = tokio::time::interval(adjusted_interval);
                            }
                            ReplayCommandResult::StepForward => {
                                let actions = player.actions_for_tick(current_tick as i64);
                                for action in &actions {
                                    apply_snake_action(&mut game_state, action, &player_map);
                                }
                                game_state.update(&mut rng);
                                current_tick += 1;

                                let alive_count = game_state.snakes.values().filter(|s| s.is_alive()).count();
                                let game_over = if total_players == 1 { alive_count == 0 } else { alive_count <= 1 };
                                let is_finished = game_over || player.is_finished();

                                let proto_state = build_snake_proto_state(&game_state, player.players(), current_tick.min(total_ticks), settings.tick_interval_ms);
                                let state_update = GameStateUpdate { state: Some(game_state_update::State::Snake(proto_state)) };
                                broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_tick.min(total_ticks), total_ticks, speed, is_finished, host_only_control).await;

                                if is_finished {
                                    return wait_for_restart_or_stop(command_rx).await;
                                }
                            }
                            ReplayCommandResult::Restart => return true,
                            ReplayCommandResult::None => {}
                        }
                    }
                }
            }
        }
    }
}
fn build_snake_proto_state(
    state: &SnakeGameState,
    players: &[crate::PlayerIdentity],
    tick: u64,
    tick_interval_ms: u32,
) -> crate::proto::snake::SnakeGameState {
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

        let is_bot = players
            .iter()
            .find(|p| p.player_id == id.to_string())
            .map(|p| p.is_bot)
            .unwrap_or(false);

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

    crate::proto::snake::SnakeGameState {
        tick,
        snakes,
        food,
        field_width: state.field_size.width as u32,
        field_height: state.field_size.height as u32,
        tick_interval_ms,
        wall_collision_mode: wall_collision_mode_proto as i32,
        dead_snake_behavior: dead_snake_behavior_proto as i32,
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
            if let Some(in_game_command::Command::Snake(snake_cmd)) = &cmd.command
                && let Some(crate::proto::snake::snake_in_game_command::Command::Turn(turn)) = &snake_cmd.command
            {
                let direction = match crate::proto::snake::Direction::try_from(turn.direction) {
                    Ok(crate::proto::snake::Direction::Up) => Direction::Up,
                    Ok(crate::proto::snake::Direction::Down) => Direction::Down,
                    Ok(crate::proto::snake::Direction::Left) => Direction::Left,
                    Ok(crate::proto::snake::Direction::Right) => Direction::Right,
                    _ => return,
                };
                if let Err(e) = game_state.set_snake_direction(player_id, direction) {
                    log!("[replay] Failed to set direction for {}: {}", player_id, e);
                }
            }
        }
        player_action_content::Content::Disconnected(_) => {
            if let Err(e) = game_state.kill_snake(player_id, DeathReason::PlayerDisconnected) {
                log!("[replay] Failed to kill snake {}: {}", player_id, e);
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



