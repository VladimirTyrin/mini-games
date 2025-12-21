use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tokio::time::{interval, Duration};
use common::{
    game_service_server::GameService,
    GameClientMessage, GameServerMessage, GameStateUpdate, GameOverNotification,
    ScoreEntry, Position,
    ClientId,
    log,
};
use crate::connection_tracker::ConnectionTracker;
use crate::game_session_manager::{GameSessionManager, SessionId};
use crate::game::Direction as GameDirection;

#[derive(Clone, Debug)]
struct GameServiceDependencies {
    tracker: ConnectionTracker,
    session_manager: GameSessionManager,
}

#[derive(Debug)]
pub struct GameServiceImpl {
    dependencies: GameServiceDependencies,
}

impl GameServiceImpl {
    pub fn new(tracker: ConnectionTracker, session_manager: GameSessionManager) -> Self {
        Self {
            dependencies: GameServiceDependencies {
                tracker,
                session_manager,
            }
        }
    }

    fn convert_direction(proto_dir: common::Direction) -> Option<GameDirection> {
        match proto_dir {
            common::Direction::Up => Some(GameDirection::Up),
            common::Direction::Down => Some(GameDirection::Down),
            common::Direction::Left => Some(GameDirection::Left),
            common::Direction::Right => Some(GameDirection::Right),
            common::Direction::Unspecified => None,
        }
    }
}

#[tonic::async_trait]
impl GameService for GameServiceImpl {
    type GameStreamStream = ReceiverStream<Result<GameServerMessage, Status>>;

    async fn game_stream(
        &self,
        request: Request<tonic::Streaming<GameClientMessage>>,
    ) -> Result<Response<Self::GameStreamStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let dependencies = self.dependencies.clone();

        tokio::spawn(async move {
            let mut client_id: Option<ClientId> = None;
            let mut session_id: Option<SessionId> = None;

            while let Some(result) = stream.next().await {
                match result {
                    Ok(msg) => {
                        let msg_client_id = ClientId::new(msg.client_id.clone());

                        if let Some(message) = msg.message {
                            match message {
                                common::game_client_message::Message::Connect(_) => {
                                    if client_id.is_some() {
                                        break;
                                    }

                                    if dependencies.tracker.add_game_client(&msg_client_id).await {
                                        if let Some(found_session_id) = dependencies.session_manager.get_session_for_client(&msg_client_id).await {
                                            client_id = Some(msg_client_id.clone());
                                            session_id = Some(found_session_id.clone());
                                            log!("Game client connected: {} to session {}", msg_client_id, found_session_id);

                                            let tx_clone = tx.clone();
                                            let session_id_clone = found_session_id;
                                            let session_manager = dependencies.session_manager.clone();

                                        tokio::spawn(async move {
                                            let mut broadcast_interval = interval(Duration::from_millis(100));

                                            loop {
                                                broadcast_interval.tick().await;

                                                if let Some((state, tick)) = session_manager.get_state(&session_id_clone).await {
                                                    let mut snakes = vec![];
                                                    for (id, snake) in &state.snakes {
                                                        let segments = snake.body.iter().map(|p| Position {
                                                            x: p.x as i32,
                                                            y: p.y as i32,
                                                        }).collect();

                                                        snakes.push(common::Snake {
                                                            client_id: id.to_string(),
                                                            segments,
                                                            alive: snake.alive,
                                                            score: snake.score,
                                                        });
                                                    }

                                                    let food: Vec<Position> = state.food_set.iter().map(|p| Position {
                                                        x: p.x as i32,
                                                        y: p.y as i32,
                                                    }).collect();

                                                    let game_state_msg = GameServerMessage {
                                                        message: Some(common::game_server_message::Message::State(
                                                            GameStateUpdate {
                                                                tick,
                                                                snakes,
                                                                food,
                                                                field_width: state.field_size.width as u32,
                                                                field_height: state.field_size.height as u32,
                                                            }
                                                        )),
                                                    };

                                                    if tx_clone.send(Ok(game_state_msg)).await.is_err() {
                                                        break;
                                                    }

                                                    if session_manager.is_game_over(&session_id_clone).await {
                                                        let scores: Vec<ScoreEntry> = state.snakes.iter().map(|(id, snake)| {
                                                            ScoreEntry {
                                                                client_id: id.to_string(),
                                                                score: snake.score,
                                                            }
                                                        }).collect();

                                                        let winner_id = scores.iter()
                                                            .max_by_key(|s| s.score)
                                                            .map(|s| s.client_id.clone())
                                                            .unwrap_or_default();

                                                        let game_over_msg = GameServerMessage {
                                                            message: Some(common::game_server_message::Message::GameOver(
                                                                GameOverNotification {
                                                                    scores,
                                                                    winner_id,
                                                                }
                                                            )),
                                                        };

                                                        let _ = tx_clone.send(Ok(game_over_msg)).await;
                                                        break;
                                                    }
                                                } else {
                                                    break;
                                                }
                                            }
                                        });
                                        } else {
                                            log!("Game connection rejected (no session found): {}", msg_client_id);
                                            dependencies.tracker.remove_game_client(&msg_client_id).await;
                                            break;
                                        }
                                    } else {
                                        log!("Game connection rejected (duplicate): {}", msg_client_id);
                                        break;
                                    }
                                }
                                common::game_client_message::Message::Disconnect(_) => {
                                    if let Some(id) = &client_id {
                                        dependencies.tracker.remove_game_client(id).await;
                                        log!("Game client disconnected: {}", id);
                                        client_id = None;
                                    }
                                    break;
                                }
                                common::game_client_message::Message::Turn(turn_cmd) => {
                                    if let (Some(id), Some(sess_id)) = (&client_id, &session_id) {
                                        if let Some(direction) = Self::convert_direction(
                                            common::Direction::try_from(turn_cmd.direction).unwrap_or(common::Direction::Unspecified)
                                        ) {
                                            let _ = dependencies.session_manager.set_direction(sess_id, id, direction).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log!("Game stream error: {}", e);
                        break;
                    }
                }
            }

            if let Some(id) = client_id {
                dependencies.tracker.remove_game_client(&id).await;
                log!("Game client disconnected (stream ended): {}", id);
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
