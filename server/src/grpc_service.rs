use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};

use crate::{log, proto::game_service::game_service_server::GameService, ClientId, ClientMessage, ServerMessage};

use crate::broadcaster::Broadcaster;
use crate::game_session_manager::GameSessionManager;
use crate::lobby_manager::LobbyManager;
use crate::message_handler::{HandleResult, MessageHandler};

#[derive(Debug)]
pub struct GrpcService {
    lobby_manager: LobbyManager,
    broadcaster: Broadcaster,
    session_manager: GameSessionManager,
}

impl GrpcService {
    pub fn new(
        lobby_manager: LobbyManager,
        broadcaster: Broadcaster,
        session_manager: GameSessionManager,
    ) -> Self {
        Self {
            lobby_manager,
            broadcaster,
            session_manager,
        }
    }
}

#[tonic::async_trait]
impl GameService for GrpcService {
    type GameStreamStream = ReceiverStream<Result<ServerMessage, Status>>;

    async fn game_stream(
        &self,
        request: Request<tonic::Streaming<ClientMessage>>,
    ) -> Result<Response<Self::GameStreamStream>, Status> {
        let mut in_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(128);

        let handler = MessageHandler::new(
            self.lobby_manager.clone(),
            self.broadcaster.clone(),
            self.session_manager.clone(),
        );

        tokio::spawn(async move {
            let mut client_id_opt: Option<ClientId> = None;

            while let Some(result) = in_stream.next().await {
                match result {
                    Ok(client_message) => {
                        match handler
                            .handle_message(client_message, &tx, &mut client_id_opt)
                            .await
                        {
                            HandleResult::Continue => {}
                            HandleResult::Disconnect => break,
                        }
                    }
                    Err(e) => {
                        log!("Stream error: {}", e);
                        break;
                    }
                }
            }

            if let Some(client_id) = &client_id_opt {
                log!("Stream ended for client: {}", client_id);
                handler.handle_client_disconnected(client_id).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
