use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use common::{
    game_service_server::GameService,
    GameClientMessage, GameServerMessage, ErrorResponse,
    ClientId,
    log,
};
use crate::connection_tracker::ConnectionTracker;
use crate::game_session_manager::{GameSessionManager, SessionId};
use crate::game::{Direction as GameDirection, DeathReason};
use crate::game_broadcaster::GameBroadcaster;

#[derive(Clone, Debug)]
struct GameServiceDependencies {
    tracker: ConnectionTracker,
    session_manager: GameSessionManager,
    broadcaster: GameBroadcaster,
}

#[derive(Debug)]
pub struct GameServiceImpl {
    dependencies: GameServiceDependencies,
}

impl GameServiceImpl {
    pub fn new(tracker: ConnectionTracker, session_manager: GameSessionManager, broadcaster: GameBroadcaster) -> Self {
        Self {
            dependencies: GameServiceDependencies {
                tracker,
                session_manager,
                broadcaster,
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

    async fn send_not_connected_error(
        tx: &tokio::sync::mpsc::Sender<Result<GameServerMessage, Status>>,
        action: &str,
    ) {
        let _ = tx.send(Ok(GameServerMessage {
            message: Some(common::game_server_message::Message::Error(ErrorResponse {
                message: format!("Not connected: cannot {}", action),
            })),
        })).await;
    }

    async fn handle_connect_message(
        dependencies: &GameServiceDependencies,
        client_id: &ClientId,
        tx: tokio::sync::mpsc::Sender<Result<GameServerMessage, Status>>,
    ) -> Option<SessionId> {
        if dependencies.tracker.add_game_client(client_id).await {
            if let Some(found_session_id) = dependencies.session_manager.get_session_for_client(client_id).await {
                log!("Game client connected: {} to session {}", client_id, found_session_id);

                dependencies.broadcaster.register(found_session_id.clone(), client_id.clone(), tx).await;

                Some(found_session_id)
            } else {
                log!("Game connection rejected (no session found): {}", client_id);
                let _ = tx.send(Ok(GameServerMessage {
                    message: Some(common::game_server_message::Message::Error(ErrorResponse {
                        message: "No game session found for your client".to_string(),
                    })),
                })).await;
                dependencies.tracker.remove_game_client(client_id).await;
                None
            }
        } else {
            log!("Game connection rejected (duplicate): {}", client_id);
            let _ = tx.send(Ok(GameServerMessage {
                message: Some(common::game_server_message::Message::Error(ErrorResponse {
                    message: "Client ID already connected".to_string(),
                })),
            })).await;
            None
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
                        if let Some(message) = msg.message {
                            match message {
                                common::game_client_message::Message::Connect(req) => {
                                    if client_id.is_some() {
                                        let _ = tx.send(Ok(GameServerMessage {
                                            message: Some(common::game_server_message::Message::Error(ErrorResponse {
                                                message: "Already connected".to_string(),
                                            })),
                                        })).await;
                                        continue;
                                    }

                                    let new_client_id = ClientId::new(req.client_id);
                                    if let Some(sess_id) = Self::handle_connect_message(&dependencies, &new_client_id, tx.clone()).await {
                                        client_id = Some(new_client_id);
                                        session_id = Some(sess_id);
                                    } else {
                                        break;
                                    }
                                }
                                common::game_client_message::Message::Disconnect(_) => {
                                    if let Some(id) = &client_id {
                                        if let Some(sess_id) = &session_id {
                                            let _ = dependencies.session_manager.kill_snake(sess_id, id, DeathReason::PlayerDisconnected).await;
                                            dependencies.broadcaster.unregister(sess_id, id).await;
                                        }
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
                                    } else {
                                        Self::send_not_connected_error(&tx, "send turn").await;
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
                if let Some(sess_id) = session_id {
                    let _ = dependencies.session_manager.kill_snake(&sess_id, &id, DeathReason::PlayerDisconnected).await;
                    dependencies.broadcaster.unregister(&sess_id, &id).await;
                    log!("Game client disconnected (stream ended): {} - snake marked as dead", id);
                }
                dependencies.tracker.remove_game_client(&id).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
