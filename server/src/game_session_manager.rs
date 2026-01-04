use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;
use common::{ClientId, LobbyId, PlayerId, BotId, log, ServerMessage, server_message, SnakePosition};
use crate::games::snake::{GameState as SnakeGameState, FieldSize, WallCollisionMode, DeadSnakeBehavior, Direction, Point, DeathReason, BotController as SnakeBotController};
use crate::games::tictactoe::game_state::{TicTacToeGameState, FirstPlayerMode, GameStatus as TicTacToeGameStatus};
use crate::games::tictactoe::bot_controller as TicTacToeBotController;
use crate::broadcaster::Broadcaster;
use crate::lobby_manager::{LobbyManager, BotType, LobbySettings};

pub type SessionId = String;

#[derive(Debug)]
enum GameStateEnum {
    Snake(SnakeGameState),
    TicTacToe(TicTacToeGameState),
}

#[derive(Debug)]
pub struct GameSessionManager {
    sessions: Arc<Mutex<HashMap<SessionId, GameSession>>>,
    client_to_session: Arc<Mutex<HashMap<ClientId, SessionId>>>,
    broadcaster: Broadcaster,
    lobby_manager: LobbyManager,
}

struct GameSession {
    state: Arc<Mutex<GameStateEnum>>,
    tick: Arc<Mutex<u64>>,
    bots: Arc<Mutex<HashMap<BotId, BotType>>>,
    initial_player_count: usize,
}

impl std::fmt::Debug for GameSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameSession")
            .field("state", &self.state)
            .field("tick", &self.tick)
            .field("bots", &self.bots)
            .field("initial_player_count", &self.initial_player_count)
            .finish()
    }
}

impl GameSessionManager {
    pub fn new(broadcaster: Broadcaster, lobby_manager: LobbyManager) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            client_to_session: Arc::new(Mutex::new(HashMap::new())),
            broadcaster,
            lobby_manager,
        }
    }

    pub async fn remove_session(&self, session_id: &SessionId) {
        let mut sessions = self.sessions.lock().await;
        sessions.remove(session_id);
        drop(sessions);

        let mut mapping = self.client_to_session.lock().await;
        mapping.retain(|_, sid| sid != session_id);

        log!("Game session removed: {}", session_id);
    }

    pub async fn create_session(
        &self,
        session_id: SessionId,
        _lobby_details: common::LobbyDetails,
    ) {
        let lobby_id = LobbyId::new(session_id.clone());
        let lobby = match self.lobby_manager.get_lobby(&lobby_id).await {
            Some(l) => l,
            None => {
                log!("Cannot create session: lobby {} not found", lobby_id);
                return;
            }
        };

        let human_players: Vec<PlayerId> = lobby.players.keys().cloned().collect();
        let bots: HashMap<BotId, BotType> = lobby.bots.clone();

        let (field_width, field_height, wall_collision_mode, dead_snake_behavior, tick_interval, max_food_count, food_spawn_probability) = match &lobby.settings {
            LobbySettings::Snake(snake_settings) => {
                let field_width = snake_settings.field_width as usize;
                let field_height = snake_settings.field_height as usize;
                let wall_collision_mode = match common::proto::snake::WallCollisionMode::try_from(snake_settings.wall_collision_mode) {
                    Ok(common::proto::snake::WallCollisionMode::Death) |
                    Ok(common::proto::snake::WallCollisionMode::Unspecified) => WallCollisionMode::Death,
                    Ok(common::proto::snake::WallCollisionMode::WrapAround) => WallCollisionMode::WrapAround,
                    _ => WallCollisionMode::Death,
                };
                let dead_snake_behavior = match common::proto::snake::DeadSnakeBehavior::try_from(snake_settings.dead_snake_behavior) {
                    Ok(common::proto::snake::DeadSnakeBehavior::StayOnField) => DeadSnakeBehavior::StayOnField,
                    Ok(common::proto::snake::DeadSnakeBehavior::Disappear) |
                    Ok(common::proto::snake::DeadSnakeBehavior::Unspecified) |
                    _ => DeadSnakeBehavior::Disappear,
                };
                let tick_interval = Duration::from_millis(snake_settings.tick_interval_ms as u64);
                let max_food_count = snake_settings.max_food_count.max(1) as usize;
                let food_spawn_probability = snake_settings.food_spawn_probability.clamp(0.001, 1.0);
                (field_width, field_height, wall_collision_mode, dead_snake_behavior, tick_interval, max_food_count, food_spawn_probability)
            }
            LobbySettings::TicTacToe(ttt_settings) => {
                Self::create_tictactoe_session(
                    session_id,
                    human_players,
                    bots,
                    ttt_settings,
                    self.sessions.clone(),
                    self.client_to_session.clone(),
                    self.broadcaster.clone(),
                    self.lobby_manager.clone(),
                    self.clone(),
                ).await;
                return;
            }
        };
        let mut sessions = self.sessions.lock().await;

        let field_size = FieldSize {
            width: field_width,
            height: field_height,
        };
        let mut game_state = SnakeGameState::new(field_size, wall_collision_mode, dead_snake_behavior, max_food_count, food_spawn_probability);

        let total_players = human_players.len() + bots.len();
        let mut idx = 0;

        for player_id in &human_players {
            let start_pos = Self::calculate_start_position(idx, total_players, field_width, field_height);
            let direction = Self::calculate_start_direction(idx, total_players);
            game_state.add_snake(player_id.clone(), start_pos, direction);
            idx += 1;
        }

        for (bot_id, _) in &bots {
            let start_pos = Self::calculate_start_position(idx, total_players, field_width, field_height);
            let direction = Self::calculate_start_direction(idx, total_players);
            game_state.add_snake(bot_id.to_player_id(), start_pos, direction);
            idx += 1;
        }

        let state = Arc::new(Mutex::new(GameStateEnum::Snake(game_state)));
        let tick = Arc::new(Mutex::new(0u64));
        let bot_count = bots.len();
        let bots_arc = Arc::new(Mutex::new(bots));

        let initial_player_count = human_players.len() + bot_count;
        let state_clone = state.clone();
        let tick_clone = tick.clone();
        let bots_clone = bots_arc.clone();
        let session_id_clone = session_id.clone();
        let broadcaster_clone = self.broadcaster.clone();
        let lobby_manager_clone = self.lobby_manager.clone();
        let session_manager_clone = self.clone();
        let human_players_clone = human_players.clone();

        let _ = tokio::spawn(async move {
            let mut tick_interval_timer = interval(tick_interval);

            loop {
                tick_interval_timer.tick().await;

                let mut state_guard = state_clone.lock().await;
                let state = match &mut *state_guard {
                    GameStateEnum::Snake(s) => s,
                    _ => {
                        log!("Invalid game state type in Snake game loop");
                        break;
                    }
                };

                let bots_map = bots_clone.lock().await;
                for (bot_id, bot_type) in bots_map.iter() {
                    if let BotType::Snake(snake_bot_type) = bot_type {
                        let player_id = bot_id.to_player_id();
                        if let Some(direction) = SnakeBotController::calculate_move(*snake_bot_type, &player_id, &state) {
                            state.set_snake_direction(&player_id, direction);
                        }
                    }
                }
                drop(bots_map);

                state.update();

                let mut tick_value = tick_clone.lock().await;
                *tick_value += 1;

                let mut snakes = vec![];
                let bots_ref = bots_clone.lock().await;
                for (id, snake) in &state.snakes {
                    let segments = snake.body.iter().map(|p| SnakePosition {
                        x: p.x as i32,
                        y: p.y as i32,
                    }).collect();

                    let is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == *id);

                    snakes.push(common::proto::snake::Snake {
                        identity: Some(common::proto::snake::PlayerIdentity {
                            player_id: id.to_string(),
                            is_bot,
                        }),
                        segments,
                        alive: snake.is_alive(),
                        score: snake.score,
                    });
                }
                drop(bots_ref);

                let food: Vec<SnakePosition> = state.food_set.iter().map(|p| SnakePosition {
                    x: p.x as i32,
                    y: p.y as i32,
                }).collect();

                let dead_snake_behavior_proto = match state.dead_snake_behavior {
                    DeadSnakeBehavior::Disappear => common::proto::snake::DeadSnakeBehavior::Disappear,
                    DeadSnakeBehavior::StayOnField => common::proto::snake::DeadSnakeBehavior::StayOnField,
                };

                let wall_collision_mode_proto = match state.wall_collision_mode {
                    WallCollisionMode::Death => common::proto::snake::WallCollisionMode::Death,
                    WallCollisionMode::WrapAround => common::proto::snake::WallCollisionMode::WrapAround,
                };

                let game_state_msg = ServerMessage {
                    message: Some(server_message::Message::GameState(
                        common::GameStateUpdate {
                            state: Some(common::game_state_update::State::Snake(
                                common::proto::snake::SnakeGameState {
                                    tick: *tick_value,
                                    snakes,
                                    food,
                                    field_width: state.field_size.width as u32,
                                    field_height: state.field_size.height as u32,
                                    tick_interval_ms: tick_interval.as_millis() as u32,
                                    wall_collision_mode: wall_collision_mode_proto as i32,
                                    dead_snake_behavior: dead_snake_behavior_proto as i32,
                                }
                            ))
                        }
                    )),
                };

                let client_ids: Vec<ClientId> = human_players_clone.iter()
                    .map(|p| ClientId::new(p.to_string()))
                    .collect();
                broadcaster_clone.broadcast_to_clients(&client_ids, game_state_msg).await;

                let alive_count = state.snakes.values().filter(|s| s.is_alive()).count();
                let game_over = if initial_player_count == 1 {
                    alive_count == 0
                } else {
                    alive_count <= 1
                };

                drop(state_guard);
                drop(tick_value);

                if game_over {
                    let mut state_guard = state_clone.lock().await;
                    let state = match &mut *state_guard {
                        GameStateEnum::Snake(s) => s,
                        _ => {
                            log!("Invalid game state type in game over handling");
                            break;
                        }
                    };
                    let bots_ref = bots_clone.lock().await;
                    let scores: Vec<common::ScoreEntry> = state.snakes.iter().map(|(id, snake)| {
                        let is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == *id);

                        common::ScoreEntry {
                            identity: Some(common::PlayerIdentity {
                                player_id: id.to_string(),
                                is_bot,
                            }),
                            score: snake.score,
                        }
                    }).collect();

                    let winner = state.snakes.iter()
                        .find(|(_, snake)| snake.is_alive())
                        .map(|(id, _)| {
                            let is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == *id);
                            common::PlayerIdentity {
                                player_id: id.to_string(),
                                is_bot,
                            }
                        });
                    drop(bots_ref);

                    let game_end_reason = state.game_end_reason
                        .map(|r| match r {
                            DeathReason::WallCollision => common::proto::snake::SnakeGameEndReason::WallCollision,
                            DeathReason::SelfCollision => common::proto::snake::SnakeGameEndReason::SelfCollision,
                            DeathReason::OtherSnakeCollision => common::proto::snake::SnakeGameEndReason::SnakeCollision,
                            DeathReason::PlayerDisconnected => common::proto::snake::SnakeGameEndReason::PlayerDisconnected,
                        })
                        .unwrap_or(common::proto::snake::SnakeGameEndReason::GameCompleted);

                    let game_over_msg = ServerMessage {
                        message: Some(server_message::Message::GameOver(
                            common::GameOverNotification {
                                scores,
                                winner,
                                reason: Some(common::game_over_notification::Reason::SnakeReason(game_end_reason as i32)),
                            }
                        )),
                    };

                    broadcaster_clone.broadcast_to_clients(&client_ids, game_over_msg).await;

                    drop(state_guard);

                    let lobby_id = LobbyId::new(session_id_clone.clone());
                    match lobby_manager_clone.end_game(&lobby_id).await {
                        Ok(_player_ids) => {
                            log!("Game ended for lobby {}, {} players in lobby", lobby_id, _player_ids.len());

                            if let Some(_lobby_details) = lobby_manager_clone.get_lobby_details(&lobby_id).await {
                                match lobby_manager_clone.get_play_again_status(&lobby_id).await {
                                    Ok(status) => {
                                        let (ready_players, pending_players, available) = match status {
                                            crate::lobby_manager::PlayAgainStatus::NotAvailable => {
                                                (vec![], vec![], false)
                                            }
                                            crate::lobby_manager::PlayAgainStatus::Available { ready_player_ids, pending_player_ids } => {
                                                let ready = ready_player_ids.iter().map(|id| common::PlayerIdentity {
                                                    player_id: id.clone(),
                                                    is_bot: false,
                                                }).collect();
                                                let pending = pending_player_ids.iter().map(|id| common::PlayerIdentity {
                                                    player_id: id.clone(),
                                                    is_bot: false,
                                                }).collect();
                                                (ready, pending, true)
                                            }
                                        };

                                        let play_again_msg = ServerMessage {
                                            message: Some(server_message::Message::PlayAgainStatus(
                                                common::PlayAgainStatusNotification {
                                                    ready_players,
                                                    pending_players,
                                                    available,
                                                }
                                            )),
                                        };

                                        broadcaster_clone.broadcast_to_clients(&client_ids, play_again_msg).await;
                                    }
                                    Err(e) => {
                                        log!("Failed to get play again status: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log!("Failed to end game for lobby {}: {}", lobby_id, e);
                        }
                    }
                    break;
                }
            }

            session_manager_clone.remove_session(&session_id_clone).await;
        });

        let session = GameSession {
            state,
            tick,
            bots: bots_arc,
            initial_player_count,
        };

        sessions.insert(session_id.clone(), session);
        drop(sessions);

        let mut mapping = self.client_to_session.lock().await;
        for player_id in &human_players {
            mapping.insert(ClientId::new(player_id.to_string()), session_id.clone());
        }

        log!("Game session created: {} with {} players ({} humans, {} bots)", session_id, human_players.len() + bot_count, human_players.len(), bot_count);
    }

    pub async fn set_direction(
        &self,
        client_id: &ClientId,
        direction: Direction,
    ) {
        let mapping = self.client_to_session.lock().await;
        if let Some(session_id) = mapping.get(client_id) {
            let sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get(session_id) {
                let mut state_guard = session.state.lock().await;
                if let GameStateEnum::Snake(state) = &mut *state_guard {
                    let player_id = PlayerId::new(client_id.to_string());
                    state.set_snake_direction(&player_id, direction);
                }
            }
        }
    }

    pub async fn kill_snake(
        &self,
        client_id: &ClientId,
        reason: DeathReason,
    ) {
        let mapping = self.client_to_session.lock().await;
        if let Some(session_id) = mapping.get(client_id) {
            let sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get(session_id) {
                let mut state_guard = session.state.lock().await;
                if let GameStateEnum::Snake(state) = &mut *state_guard {
                    let player_id = PlayerId::new(client_id.to_string());
                    state.kill_snake(&player_id, reason);
                }
            }
        }
    }

    fn calculate_start_position(index: usize, total: usize, width: usize, height: usize) -> Point {
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

        let y = height / 2;

        Point::new(x.min(width - 1), y)
    }

    fn calculate_start_direction(_index: usize, _total: usize) -> Direction {
        Direction::Up
    }

    async fn create_tictactoe_session(
        session_id: SessionId,
        human_players: Vec<PlayerId>,
        bots: HashMap<BotId, BotType>,
        ttt_settings: &common::proto::tictactoe::TicTacToeLobbySettings,
        sessions: Arc<Mutex<HashMap<SessionId, GameSession>>>,
        client_to_session: Arc<Mutex<HashMap<ClientId, SessionId>>>,
        broadcaster: Broadcaster,
        lobby_manager: LobbyManager,
        session_manager: GameSessionManager,
    ) {
        let field_width = ttt_settings.field_width as usize;
        let field_height = ttt_settings.field_height as usize;
        let win_count = ttt_settings.win_count as usize;
        let first_player_mode = match common::proto::tictactoe::FirstPlayerMode::try_from(ttt_settings.first_player) {
            Ok(common::proto::tictactoe::FirstPlayerMode::Host) => FirstPlayerMode::Host,
            Ok(common::proto::tictactoe::FirstPlayerMode::Random) |
            Ok(common::proto::tictactoe::FirstPlayerMode::Unspecified) |
            _ => FirstPlayerMode::Random,
        };

        if human_players.len() + bots.len() != 2 {
            log!("TicTacToe requires exactly 2 players, got {} humans and {} bots", human_players.len(), bots.len());
            return;
        }

        let mut all_players: Vec<PlayerId> = human_players.clone();
        for (bot_id, _) in &bots {
            all_players.push(bot_id.to_player_id());
        }

        let game_state = TicTacToeGameState::new(
            field_width,
            field_height,
            win_count,
            all_players.clone(),
            first_player_mode,
        );

        let state = Arc::new(Mutex::new(GameStateEnum::TicTacToe(game_state)));
        let tick = Arc::new(Mutex::new(0u64));
        let bot_count = bots.len();
        let bots_arc = Arc::new(Mutex::new(bots));
        let initial_player_count = human_players.len() + bot_count;

        let session = GameSession {
            state: state.clone(),
            tick,
            bots: bots_arc.clone(),
            initial_player_count,
        };

        let mut sessions_guard = sessions.lock().await;
        sessions_guard.insert(session_id.clone(), session);
        drop(sessions_guard);

        let mut mapping = client_to_session.lock().await;
        for player_id in &human_players {
            mapping.insert(ClientId::new(player_id.to_string()), session_id.clone());
        }
        drop(mapping);

        log!("TicTacToe session created: {} with {} players ({} humans, {} bots)", session_id, human_players.len() + bot_count, human_players.len(), bot_count);

        let state_clone = state.clone();
        let bots_clone = bots_arc.clone();
        let session_id_clone = session_id.clone();
        let human_players_clone = human_players.clone();

        let _ = tokio::spawn(async move {
            Self::broadcast_tictactoe_state(&state_clone, &bots_clone, &human_players_clone, &broadcaster).await;

            {
                let bots = bots_clone.lock().await;
                log!("TicTacToe game loop starting with {} bots", bots.len());
                for (bot_id, bot_type) in bots.iter() {
                    log!("Bot: {} (player_id: {}) - type: {:?}", bot_id, bot_id.to_player_id(), bot_type);
                }
            }

            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;

                let mut state_guard = state_clone.lock().await;
                let state = match &mut *state_guard {
                    GameStateEnum::TicTacToe(s) => s,
                    _ => {
                        log!("Invalid game state type in TicTacToe game loop");
                        break;
                    }
                };

                if state.status != TicTacToeGameStatus::InProgress {
                    drop(state_guard);
                    break;
                }

                let current_player = state.current_player.clone();
                let bots_map = bots_clone.lock().await;
                let is_bot_turn = bots_map.iter().any(|(bot_id, _)| bot_id.to_player_id() == current_player);

                if is_bot_turn {
                    let bot_type = bots_map.iter()
                        .find(|(bot_id, _)| bot_id.to_player_id() == current_player)
                        .and_then(|(_, bot_type)| match bot_type {
                            BotType::TicTacToe(ttt_bot) => Some(*ttt_bot),
                            _ => None,
                        });

                    if let Some(bot_type) = bot_type {
                        log!("Bot turn detected, calculating move with bot type: {:?}", bot_type);
                        if let Some((x, y)) = TicTacToeBotController::calculate_move(bot_type, &state) {
                            log!("Bot placing mark at ({}, {})", x, y);
                            if let Err(e) = state.place_mark(&current_player, x, y) {
                                log!("Bot move failed: {}", e);
                            } else {
                                log!("Bot move succeeded");
                                drop(state_guard);
                                drop(bots_map);

                                Self::broadcast_tictactoe_state(&state_clone, &bots_clone, &human_players_clone, &broadcaster).await;

                                let mut state_guard = state_clone.lock().await;
                                let state = match &mut *state_guard {
                                    GameStateEnum::TicTacToe(s) => s,
                                    _ => break,
                                };

                                if state.status != TicTacToeGameStatus::InProgress {
                                    drop(state_guard);
                                    break;
                                }

                                drop(state_guard);
                                continue;
                            }
                        }
                    }
                }

                drop(bots_map);
                drop(state_guard);
            }

            let mut state_guard = state_clone.lock().await;
            let state = match &mut *state_guard {
                GameStateEnum::TicTacToe(s) => s,
                _ => {
                    log!("Invalid game state type in game over handling");
                    return;
                }
            };

            let bots_ref = bots_clone.lock().await;
            let scores: Vec<common::ScoreEntry> = all_players.iter().map(|player_id| {
                let is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == *player_id);
                let score = if state.get_winner().as_ref() == Some(player_id) {
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

            let winner = state.get_winner().map(|player_id| {
                let is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == player_id);
                common::PlayerIdentity {
                    player_id: player_id.to_string(),
                    is_bot,
                }
            });
            drop(bots_ref);

            let game_end_reason = match state.status {
                TicTacToeGameStatus::XWon | TicTacToeGameStatus::OWon => {
                    common::proto::tictactoe::TicTacToeGameEndReason::TictactoeGameEndReasonWin
                }
                TicTacToeGameStatus::Draw => {
                    common::proto::tictactoe::TicTacToeGameEndReason::TictactoeGameEndReasonDraw
                }
                _ => common::proto::tictactoe::TicTacToeGameEndReason::TictactoeGameEndReasonUnspecified,
            };

            let game_over_msg = ServerMessage {
                message: Some(server_message::Message::GameOver(
                    common::GameOverNotification {
                        scores,
                        winner,
                        reason: Some(common::game_over_notification::Reason::TictactoeReason(game_end_reason as i32)),
                    }
                )),
            };

            let client_ids: Vec<ClientId> = human_players_clone.iter()
                .map(|p| ClientId::new(p.to_string()))
                .collect();
            broadcaster.broadcast_to_clients(&client_ids, game_over_msg).await;

            drop(state_guard);

            let lobby_id = LobbyId::new(session_id_clone.clone());
            match lobby_manager.end_game(&lobby_id).await {
                Ok(_player_ids) => {
                    log!("TicTacToe game ended for lobby {}, {} players in lobby", lobby_id, _player_ids.len());

                    if let Some(_lobby_details) = lobby_manager.get_lobby_details(&lobby_id).await {
                        match lobby_manager.get_play_again_status(&lobby_id).await {
                            Ok(status) => {
                                let (ready_players, pending_players, available) = match status {
                                    crate::lobby_manager::PlayAgainStatus::NotAvailable => {
                                        (vec![], vec![], false)
                                    }
                                    crate::lobby_manager::PlayAgainStatus::Available { ready_player_ids, pending_player_ids } => {
                                        let ready = ready_player_ids.iter().map(|id| common::PlayerIdentity {
                                            player_id: id.clone(),
                                            is_bot: false,
                                        }).collect();
                                        let pending = pending_player_ids.iter().map(|id| common::PlayerIdentity {
                                            player_id: id.clone(),
                                            is_bot: false,
                                        }).collect();
                                        (ready, pending, true)
                                    }
                                };

                                let play_again_msg = ServerMessage {
                                    message: Some(server_message::Message::PlayAgainStatus(
                                        common::PlayAgainStatusNotification {
                                            ready_players,
                                            pending_players,
                                            available,
                                        }
                                    )),
                                };

                                broadcaster.broadcast_to_clients(&client_ids, play_again_msg).await;
                            }
                            Err(e) => {
                                log!("Failed to get play again status: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    log!("Failed to end game for lobby {}: {}", lobby_id, e);
                }
            }

            session_manager.remove_session(&session_id_clone).await;
        });
    }

    async fn broadcast_tictactoe_state(
        state: &Arc<Mutex<GameStateEnum>>,
        bots: &Arc<Mutex<HashMap<BotId, BotType>>>,
        human_players: &[PlayerId],
        broadcaster: &Broadcaster,
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

        let client_ids: Vec<ClientId> = human_players.iter()
            .map(|p| ClientId::new(p.to_string()))
            .collect();
        broadcaster.broadcast_to_clients(&client_ids, game_state_msg).await;
    }

    pub async fn place_mark(
        &self,
        client_id: &ClientId,
        x: u32,
        y: u32,
    ) {
        let mapping = self.client_to_session.lock().await;
        let session_id = match mapping.get(client_id) {
            Some(id) => id.clone(),
            None => return,
        };
        drop(mapping);

        let sessions = self.sessions.lock().await;
        let (state_arc, bots_arc) = match sessions.get(&session_id) {
            Some(session) => (session.state.clone(), session.bots.clone()),
            None => return,
        };
        drop(sessions);

        let mut state_guard = state_arc.lock().await;
        if let GameStateEnum::TicTacToe(state) = &mut *state_guard {
            let player_id = PlayerId::new(client_id.to_string());
            if let Err(e) = state.place_mark(&player_id, x as usize, y as usize) {
                log!("Failed to place mark: {}", e);
                return;
            }
        } else {
            return;
        }
        drop(state_guard);

        let human_players = match self.lobby_manager.get_lobby(&LobbyId::new(session_id)).await {
            Some(lobby) => lobby.players.keys().cloned().collect(),
            None => vec![],
        };

        Self::broadcast_tictactoe_state(&state_arc, &bots_arc, &human_players, &self.broadcaster).await;
    }
}

impl Clone for GameSessionManager {
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(),
            client_to_session: self.client_to_session.clone(),
            broadcaster: self.broadcaster.clone(),
            lobby_manager: self.lobby_manager.clone(),
        }
    }
}
