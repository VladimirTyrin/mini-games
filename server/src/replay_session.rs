use std::collections::HashMap;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::games::numbers_match::{self as nm, NumbersMatchGameState, HintMode, position_from_index};
use crate::games::puzzle2048::{Puzzle2048GameState, Direction as Puzzle2048Direction};
use crate::games::snake::{
    DeadSnakeBehavior, DeathReason, Direction, FieldSize, SnakeGameState, WallCollisionMode, Point,
};
use crate::games::stack_attack::{HorizontalDirection as StackAttackDirection, StackAttackGameState};
use crate::games::tictactoe::{FirstPlayerMode, GameStatus, TicTacToeGameState};
use crate::games::SessionRng;
use crate::replay::file_io::load_replay_from_bytes;
use crate::replay::ReplayPlayer;
use crate::{
    game_state_update, in_game_command, lobby_settings, log, player_action_content,
    ClientId, GameStateUpdate, InReplayCommand, PlayerAction, PlayerId, ReplayGame,
    ReplayStateNotification, ReplayV1, ServerMessage, SnakePosition, server_message,
    in_replay_command,
};

pub struct ReplaySessionHandle {
    pub command_tx: mpsc::UnboundedSender<ReplaySessionCommand>,
    pub host_id: ClientId,
    pub host_only_control: bool,
}

pub enum ReplaySessionCommand {
    ReplayCommand(InReplayCommand),
}

pub fn parse_replay(replay_bytes: Vec<u8>) -> Result<ReplayV1, String> {
    load_replay_from_bytes(&replay_bytes).map_err(|e| format!("Failed to parse replay: {}", e))
}

pub fn replay_game_type(replay: &ReplayV1) -> Result<ReplayGame, String> {
    let player = ReplayPlayer::new(replay.clone());
    let game = player.game();
    if matches!(game, ReplayGame::Unspecified) {
        return Err("Unknown game type in replay".to_string());
    }
    Ok(game)
}

pub fn replay_game_type_name(game: ReplayGame) -> &'static str {
    match game {
        ReplayGame::Snake => "Snake",
        ReplayGame::Tictactoe => "TicTacToe",
        ReplayGame::NumbersMatch => "NumbersMatch",
        ReplayGame::StackAttack => "StackAttack",
        ReplayGame::Puzzle2048 => "Puzzle2048",
        ReplayGame::Unspecified => "Unknown",
    }
}

pub async fn run_replay_session(
    replay: ReplayV1,
    mut command_rx: mpsc::UnboundedReceiver<ReplaySessionCommand>,
    viewers: Vec<ClientId>,
    host_only_control: bool,
    broadcaster: crate::broadcaster::Broadcaster,
) {
    let player = ReplayPlayer::new(replay.clone());
    let game = player.game();

    loop {
        let player = ReplayPlayer::new(replay.clone());

        let should_restart = match game {
            ReplayGame::Snake => {
                run_snake_replay(player, &mut command_rx, &viewers, host_only_control, &broadcaster).await
            }
            ReplayGame::Tictactoe => {
                run_tictactoe_replay(player, &mut command_rx, &viewers, host_only_control, &broadcaster).await
            }
            ReplayGame::NumbersMatch => {
                run_numbers_match_replay(player, &mut command_rx, &viewers, host_only_control, &broadcaster).await
            }
            ReplayGame::StackAttack => {
                run_stack_attack_replay(player, &mut command_rx, &viewers, host_only_control, &broadcaster).await
            }
            ReplayGame::Puzzle2048 => {
                run_puzzle2048_replay(player, &mut command_rx, &viewers, host_only_control, &broadcaster).await
            }
            ReplayGame::Unspecified => false,
        };

        if !should_restart {
            break;
        }
    }
}

async fn broadcast_state_and_replay_info(
    broadcaster: &crate::broadcaster::Broadcaster,
    viewers: &[ClientId],
    state_update: GameStateUpdate,
    is_paused: bool,
    current_tick: u64,
    total_ticks: u64,
    speed: f32,
    is_finished: bool,
    host_only_control: bool,
) {
    let game_msg = ServerMessage {
        message: Some(server_message::Message::GameState(state_update)),
    };
    broadcaster.broadcast_to_clients(viewers, game_msg).await;

    let replay_msg = ServerMessage {
        message: Some(server_message::Message::ReplayState(ReplayStateNotification {
            is_paused,
            current_tick,
            total_ticks,
            speed,
            is_finished,
            host_only_control,
        })),
    };
    broadcaster.broadcast_to_clients(viewers, replay_msg).await;
}

async fn wait_for_restart_or_stop(
    command_rx: &mut mpsc::UnboundedReceiver<ReplaySessionCommand>,
) -> bool {
    loop {
        match command_rx.recv().await {
            Some(ReplaySessionCommand::ReplayCommand(cmd)) => {
                if let Some(inner) = &cmd.command {
                    match inner {
                        in_replay_command::Command::Restart(_) => return true,
                        _ => continue,
                    }
                }
            }
            None => return false,
        }
    }
}

fn handle_replay_command(
    cmd: &InReplayCommand,
    is_paused: &mut bool,
    speed: &mut f32,
) -> ReplayCommandResult {
    let Some(inner) = &cmd.command else {
        return ReplayCommandResult::None;
    };

    match inner {
        in_replay_command::Command::Pause(_) => {
            *is_paused = true;
            ReplayCommandResult::StateChanged
        }
        in_replay_command::Command::Resume(_) => {
            *is_paused = false;
            ReplayCommandResult::StateChanged
        }
        in_replay_command::Command::SetSpeed(s) => {
            *speed = s.speed.clamp(0.25, 4.0);
            ReplayCommandResult::SpeedChanged
        }
        in_replay_command::Command::StepForward(_) => {
            if *is_paused {
                ReplayCommandResult::StepForward
            } else {
                ReplayCommandResult::None
            }
        }
        in_replay_command::Command::Restart(_) => ReplayCommandResult::Restart,
    }
}

enum ReplayCommandResult {
    None,
    StateChanged,
    SpeedChanged,
    StepForward,
    Restart,
}

async fn run_snake_replay(
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

async fn run_tictactoe_replay(
    mut player: ReplayPlayer,
    command_rx: &mut mpsc::UnboundedReceiver<ReplaySessionCommand>,
    viewers: &[ClientId],
    host_only_control: bool,
    broadcaster: &crate::broadcaster::Broadcaster,
) -> bool {
    let settings = match player.lobby_settings() {
        Some(lobby_settings::Settings::Tictactoe(s)) => *s,
        _ => return false,
    };

    let players = player.players();
    if players.len() != 2 {
        return false;
    }

    let player_ids: Vec<PlayerId> = players.iter().map(|p| PlayerId::new(p.player_id.clone())).collect();
    let player_map: HashMap<i32, PlayerId> = players
        .iter()
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
    let mut speed = 1.0_f32;
    let base_delay_ms = 500.0;

    let state_update = build_tictactoe_state(&game_state, players);
    broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_action, total_actions, speed, false, host_only_control).await;

    loop {
        let current_delay = Duration::from_millis((base_delay_ms / speed) as u64);

        tokio::select! {
            _ = tokio::time::sleep(current_delay) => {
                if is_paused {
                    continue;
                }

                if let Some(action) = player.next_action() {
                    apply_tictactoe_action(&mut game_state, action, &player_map);
                    current_action += 1;
                }

                let is_finished = player.is_finished() || game_state.status != GameStatus::InProgress;
                let state_update = build_tictactoe_state(&game_state, player.players());
                broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_action.min(total_actions), total_actions, speed, is_finished, host_only_control).await;

                if is_finished {
                    return wait_for_restart_or_stop(command_rx).await;
                }
            }
            Some(cmd) = command_rx.recv() => {
                match cmd {
                    ReplaySessionCommand::ReplayCommand(replay_cmd) => {
                        match handle_replay_command(&replay_cmd, &mut is_paused, &mut speed) {
                            ReplayCommandResult::StateChanged => {
                                let state_update = build_tictactoe_state(&game_state, player.players());
                                broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_action, total_actions, speed, false, host_only_control).await;
                            }
                            ReplayCommandResult::SpeedChanged => {}
                            ReplayCommandResult::StepForward => {
                                if let Some(action) = player.next_action() {
                                    apply_tictactoe_action(&mut game_state, action, &player_map);
                                    current_action += 1;
                                }
                                let is_finished = player.is_finished() || game_state.status != GameStatus::InProgress;
                                let state_update = build_tictactoe_state(&game_state, player.players());
                                broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_action.min(total_actions), total_actions, speed, is_finished, host_only_control).await;

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

async fn run_numbers_match_replay(
    mut player: ReplayPlayer,
    command_rx: &mut mpsc::UnboundedReceiver<ReplaySessionCommand>,
    viewers: &[ClientId],
    host_only_control: bool,
    broadcaster: &crate::broadcaster::Broadcaster,
) -> bool {
    let settings = match player.lobby_settings() {
        Some(lobby_settings::Settings::NumbersMatch(s)) => *s,
        _ => return false,
    };

    let hint_mode = match crate::proto::numbers_match::HintMode::try_from(settings.hint_mode) {
        Ok(crate::proto::numbers_match::HintMode::Limited) => HintMode::Limited,
        Ok(crate::proto::numbers_match::HintMode::Unlimited) => HintMode::Unlimited,
        Ok(crate::proto::numbers_match::HintMode::Disabled) => HintMode::Disabled,
        _ => HintMode::Limited,
    };

    let mut rng = SessionRng::new(player.seed());
    let mut game_state = NumbersMatchGameState::new(&mut rng, hint_mode);

    let total_actions = player.total_actions() as u64;
    let mut current_action: u64 = 0;
    let mut is_paused = false;
    let mut speed = 1.0_f32;
    let mut pending_highlight: Option<(u32, u32)> = None;

    let base_delay_ms = 400.0;
    let highlight_delay_ms = 300.0;

    let state_update = build_numbers_match_state(&game_state);
    broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_action, total_actions, speed, false, host_only_control).await;

    loop {
        let current_delay = if pending_highlight.is_some() {
            Duration::from_millis((highlight_delay_ms / speed) as u64)
        } else {
            Duration::from_millis((base_delay_ms / speed) as u64)
        };

        tokio::select! {
            _ = tokio::time::sleep(current_delay) => {
                if is_paused {
                    continue;
                }

                if pending_highlight.is_some() {
                    if let Some(action) = player.next_action() {
                        apply_numbers_match_action(&mut game_state, action);
                        current_action += 1;
                    }
                    pending_highlight = None;

                    let is_finished = player.is_finished() || game_state.status() != nm::GameStatus::InProgress;
                    let state_update = build_numbers_match_state(&game_state);
                    broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_action.min(total_actions), total_actions, speed, is_finished, host_only_control).await;

                    if is_finished {
                        return wait_for_restart_or_stop(command_rx).await;
                    }
                } else if let Some(action) = player.peek_next_action() {
                    let highlight = extract_remove_pair_indices(action);
                    if highlight.is_some() {
                        pending_highlight = highlight;
                        let state_update = build_numbers_match_state(&game_state);
                        broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_action, total_actions, speed, false, host_only_control).await;
                    } else {
                        let action = player.next_action().unwrap();
                        apply_numbers_match_action(&mut game_state, action);
                        current_action += 1;

                        let is_finished = player.is_finished() || game_state.status() != nm::GameStatus::InProgress;
                        let state_update = build_numbers_match_state(&game_state);
                        broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_action.min(total_actions), total_actions, speed, is_finished, host_only_control).await;

                        if is_finished {
                            return wait_for_restart_or_stop(command_rx).await;
                        }
                    }
                } else {
                    let state_update = build_numbers_match_state(&game_state);
                    broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_action.min(total_actions), total_actions, speed, true, host_only_control).await;
                    return wait_for_restart_or_stop(command_rx).await;
                }
            }
            Some(cmd) = command_rx.recv() => {
                match cmd {
                    ReplaySessionCommand::ReplayCommand(replay_cmd) => {
                        match handle_replay_command(&replay_cmd, &mut is_paused, &mut speed) {
                            ReplayCommandResult::StateChanged => {
                                let state_update = build_numbers_match_state(&game_state);
                                broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_action, total_actions, speed, false, host_only_control).await;
                            }
                            ReplayCommandResult::SpeedChanged => {}
                            ReplayCommandResult::StepForward => {
                                if pending_highlight.is_some() {
                                    if let Some(action) = player.next_action() {
                                        apply_numbers_match_action(&mut game_state, action);
                                        current_action += 1;
                                    }
                                    pending_highlight = None;
                                } else if let Some(action) = player.peek_next_action() {
                                    let highlight = extract_remove_pair_indices(action);
                                    if highlight.is_some() {
                                        pending_highlight = highlight;
                                    } else {
                                        let action = player.next_action().unwrap();
                                        apply_numbers_match_action(&mut game_state, action);
                                        current_action += 1;
                                    }
                                }
                                let is_finished = player.is_finished() || game_state.status() != nm::GameStatus::InProgress;
                                let state_update = build_numbers_match_state(&game_state);
                                broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_action.min(total_actions), total_actions, speed, is_finished, host_only_control).await;

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

async fn run_stack_attack_replay(
    mut player: ReplayPlayer,
    command_rx: &mut mpsc::UnboundedReceiver<ReplaySessionCommand>,
    viewers: &[ClientId],
    host_only_control: bool,
    broadcaster: &crate::broadcaster::Broadcaster,
) -> bool {
    let players = player.players();
    if players.is_empty() {
        return false;
    }

    let player_ids: Vec<PlayerId> = players
        .iter()
        .map(|p| PlayerId::new(p.player_id.clone()))
        .collect();
    let player_map: HashMap<i32, PlayerId> = players
        .iter()
        .enumerate()
        .map(|(i, p)| (i as i32, PlayerId::new(p.player_id.clone())))
        .collect();

    let mut game_state = StackAttackGameState::new(&player_ids);
    let mut rng = SessionRng::new(player.seed());
    let tick_interval_ms = crate::games::stack_attack::settings::TICK_INTERVAL_MS as f32;
    let mut current_tick: u64 = 0;
    let mut is_paused = false;
    let mut speed = 1.0_f32;
    let total_ticks = estimate_total_ticks(&player);

    let state_update = build_stack_attack_state(&game_state, players, current_tick);
    broadcast_state_and_replay_info(
        broadcaster,
        viewers,
        state_update,
        is_paused,
        current_tick,
        total_ticks,
        speed,
        false,
        host_only_control,
    )
    .await;

    let mut tick_timer = tokio::time::interval(Duration::from_millis(tick_interval_ms as u64));

    loop {
        tokio::select! {
            _ = tick_timer.tick() => {
                if is_paused {
                    continue;
                }

                let actions = player.actions_for_tick(current_tick as i64);
                for action in &actions {
                    apply_stack_attack_action(&mut game_state, action, &player_map);
                }

                let _events = game_state.update(&mut rng);
                current_tick += 1;

                let is_finished = game_state.is_game_over() || player.is_finished();
                let state_update =
                    build_stack_attack_state(&game_state, player.players(), current_tick.min(total_ticks));
                broadcast_state_and_replay_info(
                    broadcaster,
                    viewers,
                    state_update,
                    is_paused,
                    current_tick.min(total_ticks),
                    total_ticks,
                    speed,
                    is_finished,
                    host_only_control,
                )
                .await;

                if is_finished {
                    return wait_for_restart_or_stop(command_rx).await;
                }
            }
            Some(cmd) = command_rx.recv() => {
                match cmd {
                    ReplaySessionCommand::ReplayCommand(replay_cmd) => {
                        match handle_replay_command(&replay_cmd, &mut is_paused, &mut speed) {
                            ReplayCommandResult::StateChanged => {
                                let state_update = build_stack_attack_state(&game_state, player.players(), current_tick);
                                broadcast_state_and_replay_info(
                                    broadcaster,
                                    viewers,
                                    state_update,
                                    is_paused,
                                    current_tick,
                                    total_ticks,
                                    speed,
                                    false,
                                    host_only_control,
                                )
                                .await;
                            }
                            ReplayCommandResult::SpeedChanged => {
                                let adjusted_interval = Duration::from_millis((tick_interval_ms / speed) as u64);
                                tick_timer = tokio::time::interval(adjusted_interval);
                            }
                            ReplayCommandResult::StepForward => {
                                let actions = player.actions_for_tick(current_tick as i64);
                                for action in &actions {
                                    apply_stack_attack_action(&mut game_state, action, &player_map);
                                }

                                let _events = game_state.update(&mut rng);
                                current_tick += 1;

                                let is_finished = game_state.is_game_over() || player.is_finished();
                                let state_update =
                                    build_stack_attack_state(&game_state, player.players(), current_tick.min(total_ticks));
                                broadcast_state_and_replay_info(
                                    broadcaster,
                                    viewers,
                                    state_update,
                                    is_paused,
                                    current_tick.min(total_ticks),
                                    total_ticks,
                                    speed,
                                    is_finished,
                                    host_only_control,
                                )
                                .await;

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

async fn run_puzzle2048_replay(
    mut player: ReplayPlayer,
    command_rx: &mut mpsc::UnboundedReceiver<ReplaySessionCommand>,
    viewers: &[ClientId],
    host_only_control: bool,
    broadcaster: &crate::broadcaster::Broadcaster,
) -> bool {
    let settings = match player.lobby_settings() {
        Some(lobby_settings::Settings::Puzzle2048(s)) => *s,
        _ => return false,
    };

    let mut rng = SessionRng::new(player.seed());
    let mut game_state = Puzzle2048GameState::new(
        settings.field_width as usize,
        settings.field_height as usize,
        settings.target_value,
        &mut rng,
    );

    let total_actions = player.total_actions() as u64;
    let mut current_action: u64 = 0;
    let mut is_paused = false;
    let mut speed = 1.0_f32;
    let base_delay_ms = 400.0;

    let state_update = build_puzzle2048_state(&game_state);
    broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_action, total_actions, speed, false, host_only_control).await;

    loop {
        let current_delay = Duration::from_millis((base_delay_ms / speed) as u64);

        tokio::select! {
            _ = tokio::time::sleep(current_delay) => {
                if is_paused {
                    continue;
                }

                if let Some(action) = player.next_action() {
                    apply_puzzle2048_action(&mut game_state, action, &mut rng);
                    current_action += 1;
                }

                let is_finished = player.is_finished() || game_state.status() != crate::games::puzzle2048::GameStatus::InProgress;
                let state_update = build_puzzle2048_state(&game_state);
                broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_action.min(total_actions), total_actions, speed, is_finished, host_only_control).await;

                if is_finished {
                    return wait_for_restart_or_stop(command_rx).await;
                }
            }
            Some(cmd) = command_rx.recv() => {
                match cmd {
                    ReplaySessionCommand::ReplayCommand(replay_cmd) => {
                        match handle_replay_command(&replay_cmd, &mut is_paused, &mut speed) {
                            ReplayCommandResult::StateChanged => {
                                let state_update = build_puzzle2048_state(&game_state);
                                broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_action, total_actions, speed, false, host_only_control).await;
                            }
                            ReplayCommandResult::SpeedChanged => {}
                            ReplayCommandResult::StepForward => {
                                if let Some(action) = player.next_action() {
                                    apply_puzzle2048_action(&mut game_state, action, &mut rng);
                                    current_action += 1;
                                }
                                let is_finished = player.is_finished() || game_state.status() != crate::games::puzzle2048::GameStatus::InProgress;
                                let state_update = build_puzzle2048_state(&game_state);
                                broadcast_state_and_replay_info(broadcaster, viewers, state_update, is_paused, current_action.min(total_actions), total_actions, speed, is_finished, host_only_control).await;

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

fn build_tictactoe_state(
    game_state: &TicTacToeGameState,
    players: &[crate::PlayerIdentity],
) -> GameStateUpdate {
    let player_x_is_bot = players
        .iter()
        .find(|p| p.player_id == game_state.player_x.to_string())
        .map(|p| p.is_bot)
        .unwrap_or(false);
    let player_o_is_bot = players
        .iter()
        .find(|p| p.player_id == game_state.player_o.to_string())
        .map(|p| p.is_bot)
        .unwrap_or(false);
    let current_player_is_bot = players
        .iter()
        .find(|p| p.player_id == game_state.current_player.to_string())
        .map(|p| p.is_bot)
        .unwrap_or(false);

    let proto_state = game_state.to_proto_state(player_x_is_bot, player_o_is_bot, current_player_is_bot);
    GameStateUpdate {
        state: Some(game_state_update::State::Tictactoe(proto_state)),
    }
}

fn build_numbers_match_state(game_state: &NumbersMatchGameState) -> GameStateUpdate {
    let proto_state = game_state.to_proto();
    GameStateUpdate {
        state: Some(game_state_update::State::NumbersMatch(proto_state)),
    }
}

fn build_stack_attack_state(
    game_state: &StackAttackGameState,
    players: &[crate::PlayerIdentity],
    tick: u64,
) -> GameStateUpdate {
    let bots: HashMap<crate::BotId, crate::games::BotType> = HashMap::new();
    let mut proto_state = game_state.to_proto(tick, &bots);

    for worker in &mut proto_state.workers {
        worker.is_bot = players
            .iter()
            .find(|p| p.player_id == worker.player_id)
            .map(|p| p.is_bot)
            .unwrap_or(false);
    }

    GameStateUpdate {
        state: Some(game_state_update::State::StackAttack(proto_state)),
    }
}

fn build_puzzle2048_state(game_state: &Puzzle2048GameState) -> GameStateUpdate {
    let proto_state = game_state.to_proto();
    GameStateUpdate {
        state: Some(game_state_update::State::Puzzle2048(proto_state)),
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

    if let player_action_content::Content::Command(cmd) = inner
        && let Some(in_game_command::Command::Tictactoe(ttt_cmd)) = &cmd.command
        && let Some(crate::proto::tictactoe::tic_tac_toe_in_game_command::Command::Place(place)) = &ttt_cmd.command
        && let Err(e) = game_state.place_mark(player_id, place.x as usize, place.y as usize)
    {
        log!("[replay] Failed to place mark for {} at ({}, {}): {}", player_id, place.x, place.y, e);
    }
}

fn apply_numbers_match_action(game_state: &mut NumbersMatchGameState, action: &PlayerAction) {
    let Some(content) = &action.content else {
        return;
    };

    let Some(inner) = &content.content else {
        return;
    };

    if let player_action_content::Content::Command(cmd) = inner
        && let Some(in_game_command::Command::NumbersMatch(nm_cmd)) = &cmd.command
        && let Some(nm_inner) = &nm_cmd.command
    {
        match nm_inner {
            crate::proto::numbers_match::numbers_match_in_game_command::Command::RemovePair(remove) => {
                let pos1 = position_from_index(remove.first_index);
                let pos2 = position_from_index(remove.second_index);
                let _ = game_state.remove_pair(pos1, pos2);
            }
            crate::proto::numbers_match::numbers_match_in_game_command::Command::Refill(_) => {
                let _ = game_state.refill();
            }
            crate::proto::numbers_match::numbers_match_in_game_command::Command::RequestHint(_) => {
                let _ = game_state.request_hint();
            }
        }
    }
}

fn apply_stack_attack_action(
    game_state: &mut StackAttackGameState,
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
            if let Some(in_game_command::Command::StackAttack(stack_cmd)) = &cmd.command
                && let Some(stack_inner) = &stack_cmd.command
            {
                match stack_inner {
                    crate::proto::stack_attack::stack_attack_in_game_command::Command::Move(move_cmd) => {
                        let direction = match crate::proto::stack_attack::HorizontalDirection::try_from(move_cmd.direction) {
                            Ok(crate::proto::stack_attack::HorizontalDirection::Left) => StackAttackDirection::Left,
                            Ok(crate::proto::stack_attack::HorizontalDirection::Right) => StackAttackDirection::Right,
                            _ => return,
                        };
                        let _events = game_state.handle_move(player_id, direction);
                    }
                    crate::proto::stack_attack::stack_attack_in_game_command::Command::Jump(_) => {
                        let _events = game_state.handle_jump(player_id);
                    }
                }
            }
        }
        player_action_content::Content::Disconnected(_) => {
            game_state.handle_player_disconnect();
        }
    }
}

fn apply_puzzle2048_action(
    game_state: &mut Puzzle2048GameState,
    action: &PlayerAction,
    rng: &mut SessionRng,
) {
    let Some(content) = &action.content else {
        return;
    };

    let Some(inner) = &content.content else {
        return;
    };

    if let player_action_content::Content::Command(cmd) = inner
        && let Some(in_game_command::Command::Puzzle2048(p_cmd)) = &cmd.command
        && let Some(p_inner) = &p_cmd.command
    {
        match p_inner {
            crate::proto::puzzle2048::puzzle2048_in_game_command::Command::Move(move_cmd) => {
                let direction = match crate::proto::puzzle2048::Puzzle2048Direction::try_from(move_cmd.direction) {
                    Ok(crate::proto::puzzle2048::Puzzle2048Direction::Up) => Puzzle2048Direction::Up,
                    Ok(crate::proto::puzzle2048::Puzzle2048Direction::Down) => Puzzle2048Direction::Down,
                    Ok(crate::proto::puzzle2048::Puzzle2048Direction::Left) => Puzzle2048Direction::Left,
                    Ok(crate::proto::puzzle2048::Puzzle2048Direction::Right) => Puzzle2048Direction::Right,
                    _ => return,
                };
                game_state.apply_move(direction, rng);
            }
        }
    }
}

fn extract_remove_pair_indices(action: &PlayerAction) -> Option<(u32, u32)> {
    let content = action.content.as_ref()?;
    let inner = content.content.as_ref()?;

    if let player_action_content::Content::Command(cmd) = inner
        && let Some(in_game_command::Command::NumbersMatch(nm_cmd)) = &cmd.command
        && let Some(nm_inner) = &nm_cmd.command
        && let crate::proto::numbers_match::numbers_match_in_game_command::Command::RemovePair(remove) = nm_inner
    {
        return Some((remove.first_index, remove.second_index));
    }
    None
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
