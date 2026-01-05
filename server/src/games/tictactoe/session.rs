use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use common::{ClientId, PlayerId, BotId, log, ServerMessage, server_message};
use crate::lobby_manager::BotType;
use crate::games::{SharedContext, GameSessionResult, GameOverResult, GameStateEnum};
use super::game_state::{TicTacToeGameState, FirstPlayerMode, GameStatus};
use super::bot_controller;
use super::win_detector::check_win_with_line;

pub struct TicTacToeSessionHandle {
    pub result: GameSessionResult,
    pub turn_notify: Arc<Notify>,
}

pub fn create_session(
    ctx: &SharedContext,
    settings: &common::proto::tictactoe::TicTacToeLobbySettings,
) -> Result<TicTacToeSessionHandle, String> {
    let field_width = settings.field_width as usize;
    let field_height = settings.field_height as usize;
    let win_count = settings.win_count as usize;
    let first_player_mode = match common::proto::tictactoe::FirstPlayerMode::try_from(settings.first_player) {
        Ok(common::proto::tictactoe::FirstPlayerMode::Host) => FirstPlayerMode::Host,
        Ok(common::proto::tictactoe::FirstPlayerMode::Random | common::proto::tictactoe::FirstPlayerMode::Unspecified) |
        Err(_) => FirstPlayerMode::Random,
    };

    if ctx.human_players.len() + ctx.bots.len() != 2 {
        return Err(format!(
            "TicTacToe requires exactly 2 players, got {} humans and {} bots",
            ctx.human_players.len(),
            ctx.bots.len()
        ));
    }

    let mut all_players: Vec<PlayerId> = ctx.human_players.clone();
    for bot_id in ctx.bots.keys() {
        all_players.push(bot_id.to_player_id());
    }

    let game_state = TicTacToeGameState::new(
        field_width,
        field_height,
        win_count,
        all_players,
        first_player_mode,
    );

    Ok(TicTacToeSessionHandle {
        result: GameSessionResult {
            state: Arc::new(Mutex::new(GameStateEnum::TicTacToe(game_state))),
            tick: Arc::new(Mutex::new(0u64)),
            bots: Arc::new(Mutex::new(ctx.bots.clone())),
            observers: Arc::new(Mutex::new(ctx.observers.clone())),
        },
        turn_notify: Arc::new(Notify::new()),
    })
}

pub async fn run_game_loop(
    ctx: SharedContext,
    state: Arc<Mutex<GameStateEnum>>,
    bots: Arc<Mutex<HashMap<BotId, BotType>>>,
    observers: Arc<Mutex<HashSet<PlayerId>>>,
    turn_notify: Arc<Notify>,
) -> GameOverResult {
    broadcast_state(&state, &bots, &observers, &ctx.human_players, &ctx.broadcaster).await;

    loop {
        let bots_map = bots.lock().await;
        let is_bot_turn = {
            let state_guard = state.lock().await;
            let game_state = match &*state_guard {
                GameStateEnum::TicTacToe(s) => s,
                _ => {
                    log!("Invalid game state type in TicTacToe game loop");
                    drop(bots_map);
                    break;
                }
            };

            if game_state.status != GameStatus::InProgress {
                drop(bots_map);
                break;
            }

            bots_map.iter().any(|(bot_id, _)| bot_id.to_player_id() == game_state.current_player)
        };
        drop(bots_map);

        if is_bot_turn {
            play_bot_turn(&state, &bots).await;
            broadcast_state(&state, &bots, &observers, &ctx.human_players, &ctx.broadcaster).await;
            
            tokio::task::yield_now().await;

            let state_guard = state.lock().await;
            let game_state = match &*state_guard {
                GameStateEnum::TicTacToe(s) => s,
                _ => break,
            };
            if game_state.status != GameStatus::InProgress {
                break;
            }
            drop(state_guard);
        } else {
            turn_notify.notified().await;
        }
    }

    build_game_over_result(&ctx, &state, &bots, &observers).await
}

async fn play_bot_turn(
    state: &Arc<Mutex<GameStateEnum>>,
    bots: &Arc<Mutex<HashMap<BotId, BotType>>>,
) {
    let mut state_guard = state.lock().await;
    let game_state = match &mut *state_guard {
        GameStateEnum::TicTacToe(s) => s,
        _ => return,
    };

    let current_player = game_state.current_player.clone();
    let bots_map = bots.lock().await;

    let bot_type = bots_map.iter()
        .find(|(bot_id, _)| bot_id.to_player_id() == current_player)
        .and_then(|(_, bot_type)| match bot_type {
            BotType::TicTacToe(ttt_bot) => Some(*ttt_bot),
            _ => None,
        });

    if let Some(bot_type) = bot_type {
        if let Some(pos) = bot_controller::calculate_move(bot_type, game_state) {
            if let Err(e) = game_state.place_mark(&current_player, pos.x, pos.y) {
                log!("Bot move failed: {}", e);
            }
        }
    }
}

pub async fn handle_place_mark(
    state: &Arc<Mutex<GameStateEnum>>,
    bots: &Arc<Mutex<HashMap<BotId, BotType>>>,
    observers: &Arc<Mutex<HashSet<PlayerId>>>,
    human_players: &[PlayerId],
    broadcaster: &crate::broadcaster::Broadcaster,
    turn_notify: &Arc<Notify>,
    client_id: &ClientId,
    x: u32,
    y: u32,
) -> Result<(), String> {
    let mut state_guard = state.lock().await;
    if let GameStateEnum::TicTacToe(game_state) = &mut *state_guard {
        let player_id = PlayerId::new(client_id.to_string());
        game_state.place_mark(&player_id, x as usize, y as usize)?;
    } else {
        return Err("Invalid game state type".to_string());
    }
    drop(state_guard);

    broadcast_state(state, bots, observers, human_players, broadcaster).await;
    turn_notify.notify_one();

    Ok(())
}

async fn broadcast_state(
    state: &Arc<Mutex<GameStateEnum>>,
    bots: &Arc<Mutex<HashMap<BotId, BotType>>>,
    observers: &Arc<Mutex<HashSet<PlayerId>>>,
    human_players: &[PlayerId],
    broadcaster: &crate::broadcaster::Broadcaster,
) {
    let state_guard = state.lock().await;
    let ttt_state = match &*state_guard {
        GameStateEnum::TicTacToe(s) => s,
        _ => return,
    };

    let bots_ref = bots.lock().await;
    let player_x_is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == ttt_state.player_x);
    let player_o_is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == ttt_state.player_o);
    let current_player_is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == ttt_state.current_player);

    let proto_state = ttt_state.to_proto_state(player_x_is_bot, player_o_is_bot, current_player_is_bot);
    drop(bots_ref);
    drop(state_guard);

    let game_state_msg = ServerMessage {
        message: Some(server_message::Message::GameState(
            common::GameStateUpdate {
                state: Some(common::game_state_update::State::Tictactoe(proto_state))
            }
        )),
    };

    let observers_set = observers.lock().await;
    let mut client_ids: Vec<ClientId> = human_players.iter()
        .map(|p| ClientId::new(p.to_string()))
        .collect();
    client_ids.extend(observers_set.iter().map(|p| ClientId::new(p.to_string())));
    drop(observers_set);

    broadcaster.broadcast_to_clients(&client_ids, game_state_msg).await;
}

async fn build_game_over_result(
    ctx: &SharedContext,
    state: &Arc<Mutex<GameStateEnum>>,
    bots: &Arc<Mutex<HashMap<BotId, BotType>>>,
    observers: &Arc<Mutex<HashSet<PlayerId>>>,
) -> GameOverResult {
    let state_guard = state.lock().await;
    let game_state = match &*state_guard {
        GameStateEnum::TicTacToe(s) => s,
        _ => panic!("Invalid game state type in game over handling"),
    };

    let bots_ref = bots.lock().await;

    let all_players: Vec<PlayerId> = vec![game_state.player_x.clone(), game_state.player_o.clone()];
    let scores: Vec<common::ScoreEntry> = all_players.iter().map(|player_id| {
        let is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == *player_id);
        let score = if game_state.get_winner().as_ref() == Some(player_id) {
            1
        } else {
            0
        };

        common::ScoreEntry {
            identity: Some(common::PlayerIdentity {
                player_id: player_id.to_string(),
                is_bot,
            }),
            score,
        }
    }).collect();

    let winner = game_state.get_winner().map(|player_id| {
        let is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == player_id);
        common::PlayerIdentity {
            player_id: player_id.to_string(),
            is_bot,
        }
    });
    drop(bots_ref);

    let game_end_reason = match game_state.status {
        GameStatus::XWon | GameStatus::OWon => {
            common::proto::tictactoe::TicTacToeGameEndReason::TictactoeGameEndReasonWin
        }
        GameStatus::Draw => {
            common::proto::tictactoe::TicTacToeGameEndReason::TictactoeGameEndReasonDraw
        }
        _ => common::proto::tictactoe::TicTacToeGameEndReason::TictactoeGameEndReasonUnspecified,
    };

    let winning_line = if matches!(game_state.status, GameStatus::XWon | GameStatus::OWon) {
        check_win_with_line(&game_state.board, game_state.win_count).map(|line| line.to_proto())
    } else {
        None
    };

    let observers_set = observers.lock().await;
    let current_observers = observers_set.clone();
    drop(observers_set);

    GameOverResult {
        session_id: ctx.session_id.clone(),
        scores,
        winner,
        game_info: common::game_over_notification::GameInfo::TictactoeInfo(
            common::proto::tictactoe::TicTacToeGameEndInfo {
                reason: game_end_reason as i32,
                winning_line,
            }
        ),
        human_players: ctx.human_players.clone(),
        observers: current_observers,
    }
}
