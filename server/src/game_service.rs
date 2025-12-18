use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use common::{
    game_service_server::GameService,
    GameClientMessage, GameServerMessage,
    ClientId,
    log,
};
use crate::connection_tracker::ConnectionTracker;

#[derive(Debug)]
pub struct GameServiceImpl {
    tracker: ConnectionTracker,
}

impl GameServiceImpl {
    pub fn new(tracker: ConnectionTracker) -> Self {
        Self { tracker }
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
        let (_tx, rx) = tokio::sync::mpsc::channel(128);
        let tracker = self.tracker.clone();

        tokio::spawn(async move {
            let mut client_id: Option<ClientId> = None;

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

                                    if tracker.add_game_client(&msg_client_id).await {
                                        client_id = Some(msg_client_id.clone());
                                        log!("Game client connected: {}", msg_client_id);
                                    } else {
                                        log!("Game connection rejected (duplicate): {}", msg_client_id);
                                        break;
                                    }
                                }
                                common::game_client_message::Message::Disconnect(_) => {
                                    if let Some(id) = &client_id {
                                        tracker.remove_game_client(id).await;
                                        log!("Game client disconnected: {}", id);
                                        client_id = None;
                                    }
                                    break;
                                }
                                _ => {}
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
                tracker.remove_game_client(&id).await;
                log!("Game client disconnected (stream ended): {}", id);
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
