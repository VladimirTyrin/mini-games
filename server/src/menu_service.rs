use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use common::{
    menu_service_server::MenuService,
    MenuClientMessage, MenuServerMessage, ErrorResponse,
    log,
};
use crate::connection_tracker::ConnectionTracker;

#[derive(Debug)]
pub struct MenuServiceImpl {
    tracker: ConnectionTracker,
}

impl MenuServiceImpl {
    pub fn new(tracker: ConnectionTracker) -> Self {
        Self { tracker }
    }
}

#[tonic::async_trait]
impl MenuService for MenuServiceImpl {
    type MenuStreamStream = ReceiverStream<Result<MenuServerMessage, Status>>;

    async fn menu_stream(
        &self,
        request: Request<tonic::Streaming<MenuClientMessage>>,
    ) -> Result<Response<Self::MenuStreamStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let tracker = self.tracker.clone();

        tokio::spawn(async move {
            let mut client_id: Option<String> = None;

            while let Some(result) = stream.next().await {
                match result {
                    Ok(msg) => {
                        if let Some(message) = msg.message {
                            match message {
                                common::menu_client_message::Message::Connect(_) => {
                                    if client_id.is_some() {
                                        let _ = tx.send(Ok(MenuServerMessage {
                                            message: Some(common::menu_server_message::Message::Error(ErrorResponse {
                                                message: "Already connected".to_string(),
                                            })),
                                        })).await;
                                        continue;
                                    }

                                    if tracker.add_menu_client(&msg.client_id).await {
                                        client_id = Some(msg.client_id.clone());
                                        log!("Menu client connected: {}", msg.client_id);
                                    } else {
                                        let _ = tx.send(Ok(MenuServerMessage {
                                            message: Some(common::menu_server_message::Message::Error(ErrorResponse {
                                                message: "Client ID already connected".to_string(),
                                            })),
                                        })).await;
                                        break;
                                    }
                                }
                                common::menu_client_message::Message::Disconnect(_) => {
                                    if let Some(id) = &client_id {
                                        tracker.remove_menu_client(id).await;
                                        log!("Menu client disconnected: {}", id);
                                        client_id = None;
                                    }
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        log!("Menu stream error: {}", e);
                        break;
                    }
                }
            }

            if let Some(id) = client_id {
                tracker.remove_menu_client(&id).await;
                log!("Menu client disconnected (stream ended): {}", id);
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
