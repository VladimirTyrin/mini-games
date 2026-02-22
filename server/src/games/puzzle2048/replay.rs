use std::time::Duration;

use tokio::sync::mpsc;

use crate::games::SessionRng;
use crate::games::puzzle2048::{Direction as Puzzle2048Direction, Puzzle2048GameState};
use crate::replay::ReplayPlayer;
use crate::replay::session::{
    ReplayCommandResult, ReplaySessionCommand, broadcast_state_and_replay_info,
    handle_replay_command, wait_for_restart_or_stop,
};
use crate::{
    ClientId, GameStateUpdate, PlayerAction, game_state_update, in_game_command, lobby_settings,
    player_action_content,
};
pub(crate) async fn run_replay(
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
fn build_puzzle2048_state(game_state: &Puzzle2048GameState) -> GameStateUpdate {
    let proto_state = game_state.to_proto();
    GameStateUpdate {
        state: Some(game_state_update::State::Puzzle2048(proto_state)),
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


