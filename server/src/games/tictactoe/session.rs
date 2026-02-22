use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

use crate::{
    BotId, ClientId, GameOverNotification, GameStateUpdate, InGameCommand, PlayerIdentity,
    PlayerId, ScoreEntry, game_over_notification, game_state_update, in_game_command, log,
    proto::tictactoe::{
        PlaceMarkCommand, TicTacToeGameEndInfo, TicTacToeGameEndReason, TicTacToeBotType,
        TicTacToeInGameCommand, tic_tac_toe_in_game_command,
    },
};
use crate::games::{BotType, GameBroadcaster, GameSessionConfig, SessionRng};
use crate::replay::ReplayRecorder;
use super::bot_controller::{BotInput, calculate_minimax_move, calculate_move};
use super::game_state::TicTacToeGameState;
use super::settings::TicTacToeSessionSettings;
use super::types::GameStatus;
use super::win_detector::check_win_with_line;

#[derive(Clone)]
pub struct TicTacToeSessionState {
    pub session_id: String,
    pub game_state: Arc<Mutex<TicTacToeGameState>>,
    pub rng: Arc<Mutex<SessionRng>>,
    pub bots: HashMap<BotId, BotType>,
    pub turn_notify: Arc<Notify>,
    pub replay_recorder: Option<Arc<Mutex<ReplayRecorder>>>,
    pub tick: Arc<Mutex<u64>>,
}

impl TicTacToeSessionState {
    pub fn create(
        config: &GameSessionConfig,
        settings: &TicTacToeSessionSettings,
        seed: u64,
        replay_recorder: Option<Arc<Mutex<ReplayRecorder>>>,
    ) -> Result<Self, String> {
        if config.human_players.len() + config.bots.len() != 2 {
            return Err(format!(
                "TicTacToe requires exactly 2 players, got {} humans and {} bots",
                config.human_players.len(),
                config.bots.len()
            ));
        }

        let mut rng = SessionRng::new(seed);

        let mut all_players: Vec<PlayerId> = config.human_players.clone();
        for bot_id in config.bots.keys() {
            all_players.push(bot_id.to_player_id());
        }

        let game_state = TicTacToeGameState::new(
            settings.field_width,
            settings.field_height,
            settings.win_count,
            all_players,
            settings.first_player_mode,
            &mut rng,
        );

        Ok(Self {
            session_id: config.session_id.clone(),
            game_state: Arc::new(Mutex::new(game_state)),
            rng: Arc::new(Mutex::new(rng)),
            bots: config.bots.clone(),
            turn_notify: Arc::new(Notify::new()),
            replay_recorder,
            tick: Arc::new(Mutex::new(0)),
        })
    }
}

pub struct TicTacToeSession;

impl TicTacToeSession {
    pub async fn run(
        config: GameSessionConfig,
        session_state: TicTacToeSessionState,
        broadcaster: impl GameBroadcaster,
    ) -> GameOverNotification {
        loop {
            broadcast_state(&session_state, &config, &broadcaster).await;

            let (is_game_over, is_bot_turn) = {
                let game_state = session_state.game_state.lock().await;
                let is_over = game_state.status != GameStatus::InProgress;
                let is_bot = session_state
                    .bots
                    .iter()
                    .any(|(bot_id, _)| bot_id.to_player_id() == game_state.current_player);
                (is_over, is_bot)
            };

            if is_game_over {
                break;
            }

            if is_bot_turn {
                play_bot_turn(&session_state).await;
            } else {
                session_state.turn_notify.notified().await;
            }
        }

        build_game_over_notification(&session_state).await
    }

    pub async fn handle_command(
        state: &TicTacToeSessionState,
        client_id: &ClientId,
        command: &TicTacToeInGameCommand,
    ) {
        let (x, y) = match &command.command {
            Some(tic_tac_toe_in_game_command::Command::Place(place_cmd)) => {
                (place_cmd.x, place_cmd.y)
            }
            _ => return,
        };

        let mut state_guard = state.game_state.lock().await;
        let player_id = PlayerId::new(client_id.to_string());
        match state_guard.place_mark(&player_id, x as usize, y as usize) {
            Ok(()) => {
                drop(state_guard);

                let mut tick = state.tick.lock().await;
                if let Some(ref recorder) = state.replay_recorder {
                    let mut recorder = recorder.lock().await;
                    if let Some(player_index) = recorder.find_player_index(&client_id.to_string()) {
                        let in_game_command = create_place_command(x, y);
                        recorder.record_command(*tick as i64, player_index, in_game_command);
                    }
                }
                *tick += 1;
                drop(tick);

                state.turn_notify.notify_one();
            }
            Err(e) => {
                log!("[session:{}] Player {} failed to place mark at ({}, {}): {}", state.session_id, player_id, x, y, e);
            }
        }
    }

    pub async fn handle_player_disconnect(state: &TicTacToeSessionState, client_id: &ClientId) {
        let mut game_state = state.game_state.lock().await;
        let player_id = PlayerId::new(client_id.to_string());
        if let Err(e) = game_state.forfeit(&player_id) {
            log!("[session:{}] Player {} failed to forfeit: {}", state.session_id, player_id, e);
        }
        drop(game_state);
        state.turn_notify.notify_one();
    }
}

async fn play_bot_turn(session_state: &TicTacToeSessionState) {
    let mut game_state = session_state.game_state.lock().await;

    let current_player = game_state.current_player.clone();

    let bot_type = session_state
        .bots
        .iter()
        .find(|(bot_id, _)| bot_id.to_player_id() == current_player)
        .and_then(|(_, bot_type)| match bot_type {
            BotType::TicTacToe(ttt_bot) => Some(*ttt_bot),
            _ => None,
        });

    let Some(bot_type) = bot_type else {
        return;
    };

    let bot_input = BotInput::from_game_state(&game_state);

    let calculated_move = match bot_type {
        TicTacToeBotType::TictactoeBotTypeRandom => {
            let mut rng = session_state.rng.lock().await;
            calculate_move(bot_type, bot_input, &mut rng)
        }
        TicTacToeBotType::TictactoeBotTypeMinimax => {
            drop(game_state);
            let session_id = session_state.session_id.clone();
            let result =
                tokio::task::spawn_blocking(move || calculate_minimax_move(&bot_input)).await;

            if let Ok(Some(pos)) = result {
                let mut game_state = session_state.game_state.lock().await;
                match game_state.place_mark(&current_player, pos.x, pos.y) {
                    Ok(()) => {
                        record_bot_move(session_state, &current_player, pos.x, pos.y).await;
                    }
                    Err(e) => {
                        log!("[session:{}] Bot {} failed to place mark at ({}, {}): {}", session_id, current_player, pos.x, pos.y, e);
                    }
                }
            }
            return;
        }
        _ => return,
    };

    if let Some(pos) = calculated_move {
        match game_state.place_mark(&current_player, pos.x, pos.y) {
            Ok(()) => {
                drop(game_state);
                record_bot_move(session_state, &current_player, pos.x, pos.y).await;
            }
            Err(e) => {
                log!("[session:{}] Bot {} failed to place mark at ({}, {}): {}", session_state.session_id, current_player, pos.x, pos.y, e);
            }
        }
    }
}

async fn record_bot_move(
    session_state: &TicTacToeSessionState,
    player_id: &PlayerId,
    x: usize,
    y: usize,
) {
    let mut tick = session_state.tick.lock().await;
    if let Some(ref recorder) = session_state.replay_recorder {
        let mut recorder = recorder.lock().await;
        if let Some(player_index) = recorder.find_player_index(&player_id.to_string()) {
            let command = create_place_command(x as u32, y as u32);
            recorder.record_command(*tick as i64, player_index, command);
        }
    }
    *tick += 1;
}

fn create_place_command(x: u32, y: u32) -> InGameCommand {
    InGameCommand {
        command: Some(in_game_command::Command::Tictactoe(TicTacToeInGameCommand {
            command: Some(tic_tac_toe_in_game_command::Command::Place(
                PlaceMarkCommand { x, y },
            )),
        })),
    }
}

async fn broadcast_state(
    session_state: &TicTacToeSessionState,
    config: &GameSessionConfig,
    broadcaster: &impl GameBroadcaster,
) {
    let game_state = session_state.game_state.lock().await;

    let player_x_is_bot = session_state
        .bots
        .iter()
        .any(|(bot_id, _)| bot_id.to_player_id() == game_state.player_x);
    let player_o_is_bot = session_state
        .bots
        .iter()
        .any(|(bot_id, _)| bot_id.to_player_id() == game_state.player_o);
    let current_player_is_bot = session_state
        .bots
        .iter()
        .any(|(bot_id, _)| bot_id.to_player_id() == game_state.current_player);

    let proto_state =
        game_state.to_proto_state(player_x_is_bot, player_o_is_bot, current_player_is_bot);
    drop(game_state);

    let state_update = GameStateUpdate {
        state: Some(game_state_update::State::Tictactoe(proto_state)),
    };

    let recipients = config.get_all_recipients();
    broadcaster.broadcast_state(state_update, recipients).await;
}

async fn build_game_over_notification(
    session_state: &TicTacToeSessionState,
) -> GameOverNotification {
    let game_state = session_state.game_state.lock().await;

    let all_players: Vec<PlayerId> = vec![game_state.player_x.clone(), game_state.player_o.clone()];
    let scores: Vec<ScoreEntry> = all_players
        .iter()
        .map(|player_id| {
            let is_bot = session_state
                .bots
                .iter()
                .any(|(bot_id, _)| bot_id.to_player_id() == *player_id);
            let score = if game_state.get_winner().as_ref() == Some(player_id) {
                1
            } else {
                0
            };

            ScoreEntry {
                identity: Some(PlayerIdentity {
                    player_id: player_id.to_string(),
                    is_bot,
                }),
                score,
            }
        })
        .collect();

    let winner = game_state.get_winner().map(|player_id| {
        let is_bot = session_state
            .bots
            .iter()
            .any(|(bot_id, _)| bot_id.to_player_id() == player_id);
        PlayerIdentity {
            player_id: player_id.to_string(),
            is_bot,
        }
    });

    let game_end_reason = match game_state.status {
        GameStatus::XWon | GameStatus::OWon => TicTacToeGameEndReason::TictactoeGameEndReasonWin,
        GameStatus::Draw => TicTacToeGameEndReason::TictactoeGameEndReasonDraw,
        _ => TicTacToeGameEndReason::TictactoeGameEndReasonUnspecified,
    };

    let winning_line = if matches!(game_state.status, GameStatus::XWon | GameStatus::OWon) {
        check_win_with_line(&game_state.board, game_state.win_count).map(|line| line.to_proto())
    } else {
        None
    };

    GameOverNotification {
        scores,
        winner,
        game_info: Some(game_over_notification::GameInfo::TictactoeInfo(
            TicTacToeGameEndInfo {
                reason: game_end_reason as i32,
                winning_line,
            },
        )),
    }
}
