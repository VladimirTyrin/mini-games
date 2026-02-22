use std::collections::HashMap;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::games::SessionRng;
use crate::games::tictactoe::{FirstPlayerMode, GameStatus, TicTacToeGameState};
use crate::replay::ReplayPlayer;
use crate::replay::session::{
    ReplayCommandResult, ReplaySessionCommand, broadcast_state_and_replay_info,
    handle_replay_command, wait_for_restart_or_stop,
};
use crate::{
    ClientId, GameStateUpdate, PlayerAction, PlayerId, game_state_update, in_game_command,
    lobby_settings, player_action_content, log,
};
pub(crate) async fn run_replay(
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


