use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;
use common::{ClientId, LobbyId, PlayerId, BotId, BotType, log, ServerMessage, server_message, GameStateUpdate, Position, ScoreEntry, GameOverNotification, GameEndReason};
use crate::game::{GameState, FieldSize, WallCollisionMode, DeadSnakeBehavior, Direction, Point, DeathReason};
use crate::broadcaster::Broadcaster;
use crate::lobby_manager::LobbyManager;
use crate::bot::BotController;

pub type SessionId = String;

#[derive(Debug)]
pub struct GameSessionManager {
    sessions: Arc<Mutex<HashMap<SessionId, GameSession>>>,
    client_to_session: Arc<Mutex<HashMap<ClientId, SessionId>>>,
    broadcaster: Broadcaster,
    lobby_manager: LobbyManager,
}

struct GameSession {
    state: Arc<Mutex<GameState>>,
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
        lobby_details: common::LobbyDetails,
    ) {
        let mut human_players: Vec<PlayerId> = Vec::new();
        let mut bots: HashMap<BotId, BotType> = HashMap::new();

        for player_info in &lobby_details.players {
            if let Some(identity) = &player_info.identity {
                if identity.is_bot {
                    let bot_id = BotId::new(identity.player_id.clone());
                    let bot_type = common::BotType::try_from(identity.bot_type)
                        .unwrap_or(common::BotType::Unspecified);
                    bots.insert(bot_id, bot_type);
                } else {
                    let player_id = PlayerId::new(identity.player_id.clone());
                    human_players.push(player_id);
                }
            }
        }

        let settings = lobby_details.settings.unwrap_or_default();
        let field_width = settings.field_width as usize;
        let field_height = settings.field_height as usize;
        let wall_collision_mode = match common::WallCollisionMode::try_from(settings.wall_collision_mode) {
            Ok(common::WallCollisionMode::Death) |
            Ok(common::WallCollisionMode::Unspecified) => WallCollisionMode::Death,
            Ok(common::WallCollisionMode::WrapAround) => WallCollisionMode::WrapAround,
            _ => WallCollisionMode::Death,
        };
        let dead_snake_behavior = match common::DeadSnakeBehavior::try_from(settings.dead_snake_behavior) {
            Ok(common::DeadSnakeBehavior::StayOnField) => DeadSnakeBehavior::StayOnField,
            Ok(common::DeadSnakeBehavior::Disappear) |
            Ok(common::DeadSnakeBehavior::Unspecified) |
            _ => DeadSnakeBehavior::Disappear,
        };
        let tick_interval = Duration::from_millis(settings.tick_interval_ms as u64);
        let max_food_count = settings.max_food_count.max(1) as usize;
        let food_spawn_probability = settings.food_spawn_probability.clamp(0.001, 1.0);
        let mut sessions = self.sessions.lock().await;

        let field_size = FieldSize {
            width: field_width,
            height: field_height,
        };
        let mut game_state = GameState::new(field_size, wall_collision_mode, dead_snake_behavior, max_food_count, food_spawn_probability);

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

        let state = Arc::new(Mutex::new(game_state));
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

                let mut state = state_clone.lock().await;

                let bots_map = bots_clone.lock().await;
                for (bot_id, bot_type) in bots_map.iter() {
                    let player_id = bot_id.to_player_id();
                    if let Some(direction) = BotController::calculate_move(*bot_type, &player_id, &state) {
                        state.set_snake_direction(&player_id, direction);
                    }
                }
                drop(bots_map);

                state.update();

                let mut tick_value = tick_clone.lock().await;
                *tick_value += 1;

                let mut snakes = vec![];
                let bots_ref = bots_clone.lock().await;
                for (id, snake) in &state.snakes {
                    let segments = snake.body.iter().map(|p| Position {
                        x: p.x as i32,
                        y: p.y as i32,
                    }).collect();

                    let is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == *id);
                    let bot_type = if is_bot {
                        bots_ref.iter()
                            .find(|(bot_id, _)| bot_id.to_player_id() == *id)
                            .map(|(_, bt)| *bt as i32)
                            .unwrap_or(common::BotType::Unspecified as i32)
                    } else {
                        common::BotType::Unspecified as i32
                    };

                    snakes.push(common::Snake {
                        identity: Some(common::PlayerIdentity {
                            player_id: id.to_string(),
                            is_bot,
                            bot_type,
                        }),
                        segments,
                        alive: snake.is_alive(),
                        score: snake.score,
                    });
                }
                drop(bots_ref);

                let food: Vec<Position> = state.food_set.iter().map(|p| Position {
                    x: p.x as i32,
                    y: p.y as i32,
                }).collect();

                let dead_snake_behavior_proto = match state.dead_snake_behavior {
                    DeadSnakeBehavior::Disappear => common::DeadSnakeBehavior::Disappear,
                    DeadSnakeBehavior::StayOnField => common::DeadSnakeBehavior::StayOnField,
                };

                let game_state_msg = ServerMessage {
                    message: Some(server_message::Message::State(
                        GameStateUpdate {
                            tick: *tick_value,
                            snakes,
                            food,
                            field_width: state.field_size.width as u32,
                            field_height: state.field_size.height as u32,
                            dead_snake_behavior: dead_snake_behavior_proto as i32,
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
                if game_over {
                    let bots_ref = bots_clone.lock().await;
                    let scores: Vec<ScoreEntry> = state.snakes.iter().map(|(id, snake)| {
                        let is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == *id);
                        let bot_type = if is_bot {
                            bots_ref.iter()
                                .find(|(bot_id, _)| bot_id.to_player_id() == *id)
                                .map(|(_, bt)| *bt as i32)
                                .unwrap_or(common::BotType::Unspecified as i32)
                        } else {
                            common::BotType::Unspecified as i32
                        };

                        ScoreEntry {
                            identity: Some(common::PlayerIdentity {
                                player_id: id.to_string(),
                                is_bot,
                                bot_type,
                            }),
                            score: snake.score,
                        }
                    }).collect();

                    let winner = state.snakes.iter()
                        .find(|(_, snake)| snake.is_alive())
                        .map(|(id, _)| {
                            let is_bot = bots_ref.iter().any(|(bot_id, _)| bot_id.to_player_id() == *id);
                            let bot_type = if is_bot {
                                bots_ref.iter()
                                    .find(|(bot_id, _)| bot_id.to_player_id() == *id)
                                    .map(|(_, bt)| *bt as i32)
                                    .unwrap_or(common::BotType::Unspecified as i32)
                            } else {
                                common::BotType::Unspecified as i32
                            };
                            common::PlayerIdentity {
                                player_id: id.to_string(),
                                is_bot,
                                bot_type,
                            }
                        });
                    drop(bots_ref);

                    let game_end_reason = state.game_end_reason
                        .map(|r| match r {
                            DeathReason::WallCollision => GameEndReason::WallCollision as i32,
                            DeathReason::SelfCollision => GameEndReason::SelfCollision as i32,
                            DeathReason::OtherSnakeCollision => GameEndReason::SnakeCollision as i32,
                            DeathReason::PlayerDisconnected => GameEndReason::PlayerDisconnected as i32,
                        })
                        .unwrap_or(GameEndReason::GameCompleted as i32);

                    let game_over_msg = ServerMessage {
                        message: Some(server_message::Message::GameOver(
                            GameOverNotification {
                                scores,
                                winner,
                                reason: game_end_reason,
                            }
                        )),
                    };

                    broadcaster_clone.broadcast_to_clients(&client_ids, game_over_msg).await;

                    drop(state);
                    drop(tick_value);

                    let lobby_id = LobbyId::new(session_id_clone.clone());
                    match lobby_manager_clone.end_game(&lobby_id).await {
                        Ok(_player_ids) => {
                            log!("Game ended for lobby {}, {} players in lobby", lobby_id, _player_ids.len());

                            if let Some(_lobby_details) = lobby_manager_clone.get_lobby_details(&lobby_id).await {
                                match lobby_manager_clone.get_play_again_status(&lobby_id).await {
                                    Ok(status) => {
                                        let proto_status = match status {
                                            crate::lobby_manager::PlayAgainStatus::NotAvailable => {
                                                common::play_again_status_notification::Status::NotAvailable(
                                                    common::PlayAgainNotAvailable {}
                                                )
                                            }
                                            crate::lobby_manager::PlayAgainStatus::Available { ready_player_ids, pending_player_ids } => {
                                                common::play_again_status_notification::Status::Available(
                                                    common::PlayAgainAvailable {
                                                        ready_players: ready_player_ids.iter().map(|id| common::PlayerIdentity {
                                                            player_id: id.clone(),
                                                            is_bot: false,
                                                            bot_type: common::BotType::Unspecified as i32,
                                                        }).collect(),
                                                        pending_players: pending_player_ids.iter().map(|id| common::PlayerIdentity {
                                                            player_id: id.clone(),
                                                            is_bot: false,
                                                            bot_type: common::BotType::Unspecified as i32,
                                                        }).collect(),
                                                    }
                                                )
                                            }
                                        };

                                        let play_again_msg = ServerMessage {
                                            message: Some(server_message::Message::PlayAgainStatus(
                                                common::PlayAgainStatusNotification {
                                                    status: Some(proto_status),
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

                drop(state);
                drop(tick_value);
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
                let mut state = session.state.lock().await;
                let player_id = PlayerId::new(client_id.to_string());
                state.set_snake_direction(&player_id, direction);
            }
        }
    }

    pub async fn kill_snake(
        &self,
        client_id: &ClientId,
        reason: crate::game::DeathReason,
    ) {
        let mapping = self.client_to_session.lock().await;
        if let Some(session_id) = mapping.get(client_id) {
            let sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get(session_id) {
                let mut state = session.state.lock().await;
                let player_id = PlayerId::new(client_id.to_string());
                state.kill_snake(&player_id, reason);
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
