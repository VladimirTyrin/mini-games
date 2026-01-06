use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

use crate::{
    PlayerId, BotId,
    GameStateUpdate, game_state_update, GameOverNotification, game_over_notification,
    ScoreEntry, PlayerIdentity,
    proto::tictactoe::{TicTacToeGameEndReason, TicTacToeGameEndInfo},
};
use crate::lobby::BotType;
use crate::engine::tictactoe::{TicTacToeGameState, FirstPlayerMode, GameStatus, calculate_move, BotInput, check_win_with_line};
use crate::engine::session::{GameBroadcaster, GameSessionConfig};

pub struct TicTacToeSessionState {
    pub game_state: Arc<Mutex<TicTacToeGameState>>,
    pub bots: HashMap<BotId, BotType>,
    pub turn_notify: Arc<Notify>,
}

pub struct TicTacToeSessionSettings {
    pub field_width: usize,
    pub field_height: usize,
    pub win_count: usize,
    pub first_player_mode: FirstPlayerMode,
}

impl From<&crate::proto::tictactoe::TicTacToeLobbySettings> for TicTacToeSessionSettings {
    fn from(settings: &crate::proto::tictactoe::TicTacToeLobbySettings) -> Self {
        let first_player_mode = match crate::proto::tictactoe::FirstPlayerMode::try_from(settings.first_player) {
            Ok(crate::proto::tictactoe::FirstPlayerMode::Host) => FirstPlayerMode::Host,
            Ok(crate::proto::tictactoe::FirstPlayerMode::Random | crate::proto::tictactoe::FirstPlayerMode::Unspecified) |
            Err(_) => FirstPlayerMode::Random,
        };

        Self {
            field_width: settings.field_width as usize,
            field_height: settings.field_height as usize,
            win_count: settings.win_count as usize,
            first_player_mode,
        }
    }
}

pub fn create_session(
    config: &GameSessionConfig,
    settings: &TicTacToeSessionSettings,
) -> Result<TicTacToeSessionState, String> {
    if config.human_players.len() + config.bots.len() != 2 {
        return Err(format!(
            "TicTacToe requires exactly 2 players, got {} humans and {} bots",
            config.human_players.len(),
            config.bots.len()
        ));
    }

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
    );

    Ok(TicTacToeSessionState {
        game_state: Arc::new(Mutex::new(game_state)),
        bots: config.bots.clone(),
        turn_notify: Arc::new(Notify::new()),
    })
}

pub async fn run_game_loop<B: GameBroadcaster>(
    config: GameSessionConfig,
    session_state: TicTacToeSessionState,
    broadcaster: B,
) -> GameOverNotification {
    loop {
        broadcast_state(&session_state, &config, &broadcaster).await;

        let (is_game_over, is_bot_turn) = {
            let game_state = session_state.game_state.lock().await;
            let is_over = game_state.status != GameStatus::InProgress;
            let is_bot = session_state.bots.iter().any(|(bot_id, _)| bot_id.to_player_id() == game_state.current_player);
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

    build_game_over_notification(&config, &session_state).await
}

async fn play_bot_turn(session_state: &TicTacToeSessionState) {
    let (bot_input, bot_type, current_player) = {
        let game_state = session_state.game_state.lock().await;

        let current_player = game_state.current_player.clone();

        let bot_type = session_state.bots.iter()
            .find(|(bot_id, _)| bot_id.to_player_id() == current_player)
            .and_then(|(_, bot_type)| match bot_type {
                BotType::TicTacToe(ttt_bot) => Some(*ttt_bot),
                _ => None,
            });

        match bot_type {
            Some(bt) => (BotInput::from_game_state(&game_state), bt, current_player),
            None => return,
        }
    };

    let calculated_move = tokio::task::spawn_blocking(move || {
        calculate_move(bot_type, bot_input)
    }).await;

    if let Ok(Some(pos)) = calculated_move {
        let mut game_state = session_state.game_state.lock().await;
        let _ = game_state.place_mark(&current_player, pos.x, pos.y);
    }
}

async fn broadcast_state<B: GameBroadcaster>(
    session_state: &TicTacToeSessionState,
    config: &GameSessionConfig,
    broadcaster: &B,
) {
    let game_state = session_state.game_state.lock().await;

    let player_x_is_bot = session_state.bots.iter().any(|(bot_id, _)| bot_id.to_player_id() == game_state.player_x);
    let player_o_is_bot = session_state.bots.iter().any(|(bot_id, _)| bot_id.to_player_id() == game_state.player_o);
    let current_player_is_bot = session_state.bots.iter().any(|(bot_id, _)| bot_id.to_player_id() == game_state.current_player);

    let proto_state = game_state.to_proto_state(player_x_is_bot, player_o_is_bot, current_player_is_bot);
    drop(game_state);

    let state_update = GameStateUpdate {
        state: Some(game_state_update::State::Tictactoe(proto_state)),
    };

    let recipients = config.get_all_recipients();
    broadcaster.broadcast_state(state_update, recipients).await;
}

async fn build_game_over_notification(
    _config: &GameSessionConfig,
    session_state: &TicTacToeSessionState,
) -> GameOverNotification {
    let game_state = session_state.game_state.lock().await;

    let all_players: Vec<PlayerId> = vec![game_state.player_x.clone(), game_state.player_o.clone()];
    let scores: Vec<ScoreEntry> = all_players.iter().map(|player_id| {
        let is_bot = session_state.bots.iter().any(|(bot_id, _)| bot_id.to_player_id() == *player_id);
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
    }).collect();

    let winner = game_state.get_winner().map(|player_id| {
        let is_bot = session_state.bots.iter().any(|(bot_id, _)| bot_id.to_player_id() == player_id);
        PlayerIdentity {
            player_id: player_id.to_string(),
            is_bot,
        }
    });

    let game_end_reason = match game_state.status {
        GameStatus::XWon | GameStatus::OWon => {
            TicTacToeGameEndReason::TictactoeGameEndReasonWin
        }
        GameStatus::Draw => {
            TicTacToeGameEndReason::TictactoeGameEndReasonDraw
        }
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
            }
        )),
    }
}
