use std::time::Duration;

use tokio::sync::mpsc;

use crate::games::numbers_match::{self as nm, HintMode, NumbersMatchGameState, position_from_index};
use crate::games::SessionRng;
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
fn build_numbers_match_state(game_state: &NumbersMatchGameState) -> GameStateUpdate {
    let proto_state = game_state.to_proto();
    GameStateUpdate {
        state: Some(game_state_update::State::NumbersMatch(proto_state)),
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


