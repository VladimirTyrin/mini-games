use std::collections::HashMap;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::games::SessionRng;
use crate::games::stack_attack::{HorizontalDirection as StackAttackDirection, StackAttackGameState};
use crate::replay::ReplayPlayer;
use crate::replay::session::{
    ReplayCommandResult, ReplaySessionCommand, broadcast_state_and_replay_info,
    estimate_total_ticks, handle_replay_command, wait_for_restart_or_stop,
};
use crate::{
    ClientId, GameStateUpdate, PlayerAction, PlayerId, game_state_update, in_game_command,
    player_action_content,
};
pub(crate) async fn run_replay(
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


