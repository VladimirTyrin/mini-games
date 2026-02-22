use tokio::sync::mpsc;

use crate::replay::ReplayPlayer;
use crate::replay::file_io::load_replay_from_bytes;
use crate::{
    ClientId, GameStateUpdate, InReplayCommand, ReplayGame, ReplayStateNotification, ReplayV1,
    ServerMessage, in_replay_command, server_message,
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
                crate::games::snake::replay::run_replay(
                    player,
                    &mut command_rx,
                    &viewers,
                    host_only_control,
                    &broadcaster,
                )
                .await
            }
            ReplayGame::Tictactoe => {
                crate::games::tictactoe::replay::run_replay(
                    player,
                    &mut command_rx,
                    &viewers,
                    host_only_control,
                    &broadcaster,
                )
                .await
            }
            ReplayGame::NumbersMatch => {
                crate::games::numbers_match::replay::run_replay(
                    player,
                    &mut command_rx,
                    &viewers,
                    host_only_control,
                    &broadcaster,
                )
                .await
            }
            ReplayGame::StackAttack => {
                crate::games::stack_attack::replay::run_replay(
                    player,
                    &mut command_rx,
                    &viewers,
                    host_only_control,
                    &broadcaster,
                )
                .await
            }
            ReplayGame::Puzzle2048 => {
                crate::games::puzzle2048::replay::run_replay(
                    player,
                    &mut command_rx,
                    &viewers,
                    host_only_control,
                    &broadcaster,
                )
                .await
            }
            ReplayGame::Unspecified => false,
        };

        if !should_restart {
            break;
        }
    }
}

pub(crate) async fn broadcast_state_and_replay_info(
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

pub(crate) async fn wait_for_restart_or_stop(
    command_rx: &mut mpsc::UnboundedReceiver<ReplaySessionCommand>,
) -> bool {
    loop {
        match command_rx.recv().await {
            Some(ReplaySessionCommand::ReplayCommand(cmd)) => {
                if let Some(in_replay_command::Command::Restart(_)) = &cmd.command {
                    return true;
                }
            }
            None => return false,
        }
    }
}

pub(crate) fn handle_replay_command(
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

pub(crate) enum ReplayCommandResult {
    None,
    StateChanged,
    SpeedChanged,
    StepForward,
    Restart,
}

pub(crate) fn estimate_total_ticks(player: &ReplayPlayer) -> u64 {
    let mut max_tick = 0_i64;
    for action in &player.replay_ref().actions {
        if action.tick > max_tick {
            max_tick = action.tick;
        }
    }
    if max_tick <= 0 {
        0
    } else {
        max_tick as u64 + 1
    }
}
