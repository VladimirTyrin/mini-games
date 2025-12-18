use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use common::{
    menu_service_server::MenuService,
    MenuClientMessage, MenuServerMessage, ErrorResponse,
    ClientId, LobbyId,
    log,
};
use crate::connection_tracker::ConnectionTracker;
use crate::lobby_manager::LobbyManager;

#[derive(Debug)]
pub struct MenuServiceImpl {
    tracker: ConnectionTracker,
    lobby_manager: LobbyManager,
}

impl MenuServiceImpl {
    pub fn new(tracker: ConnectionTracker, lobby_manager: LobbyManager) -> Self {
        Self { tracker, lobby_manager }
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

        let lobby_manager = self.lobby_manager.clone();

        tokio::spawn(async move {
            let mut client_id: Option<ClientId> = None;

            while let Some(result) = stream.next().await {
                match result {
                    Ok(msg) => {
                        let msg_client_id = ClientId::new(msg.client_id.clone());

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

                                    if tracker.add_menu_client(&msg_client_id).await {
                                        client_id = Some(msg_client_id.clone());
                                        log!("Menu client connected: {}", msg_client_id);
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
                                        lobby_manager.cleanup_client(id).await;
                                        log!("Menu client disconnected: {}", id);
                                        client_id = None;
                                    }
                                    break;
                                }
                                common::menu_client_message::Message::ListLobbies(_) => {
                                    if client_id.is_none() {
                                        continue;
                                    }

                                    let lobbies = lobby_manager.list_lobbies().await;
                                    let _ = tx.send(Ok(MenuServerMessage {
                                        message: Some(common::menu_server_message::Message::LobbyList(
                                            common::LobbyListResponse { lobbies }
                                        )),
                                    })).await;
                                }
                                common::menu_client_message::Message::CreateLobby(req) => {
                                    if client_id.is_none() {
                                        continue;
                                    }

                                    match lobby_manager.create_lobby(
                                        req.lobby_name,
                                        req.max_players,
                                        req.settings.unwrap_or_default(),
                                        msg_client_id.clone(),
                                    ).await {
                                        Ok(details) => {
                                            log!("Lobby created: {} by {}", details.lobby_id, msg_client_id);
                                            let _ = tx.send(Ok(MenuServerMessage {
                                                message: Some(common::menu_server_message::Message::LobbyUpdate(
                                                    common::LobbyUpdateNotification { lobby: Some(details) }
                                                )),
                                            })).await;
                                        }
                                        Err(e) => {
                                            let _ = tx.send(Ok(MenuServerMessage {
                                                message: Some(common::menu_server_message::Message::Error(
                                                    ErrorResponse { message: e }
                                                )),
                                            })).await;
                                        }
                                    }
                                }
                                common::menu_client_message::Message::JoinLobby(req) => {
                                    if client_id.is_none() {
                                        continue;
                                    }

                                    match lobby_manager.join_lobby(
                                        LobbyId::new(req.lobby_id),
                                        msg_client_id.clone(),
                                    ).await {
                                        Ok(details) => {
                                            log!("{} joined lobby {}", msg_client_id, details.lobby_id);
                                            let _ = tx.send(Ok(MenuServerMessage {
                                                message: Some(common::menu_server_message::Message::LobbyUpdate(
                                                    common::LobbyUpdateNotification { lobby: Some(details) }
                                                )),
                                            })).await;
                                        }
                                        Err(e) => {
                                            let _ = tx.send(Ok(MenuServerMessage {
                                                message: Some(common::menu_server_message::Message::Error(
                                                    ErrorResponse { message: e }
                                                )),
                                            })).await;
                                        }
                                    }
                                }
                                common::menu_client_message::Message::LeaveLobby(_) => {
                                    if client_id.is_none() {
                                        continue;
                                    }

                                    match lobby_manager.leave_lobby(&msg_client_id).await {
                                        Ok(details_opt) => {
                                            log!("{} left lobby", msg_client_id);
                                            if let Some(details) = details_opt {
                                                let _ = tx.send(Ok(MenuServerMessage {
                                                    message: Some(common::menu_server_message::Message::LobbyUpdate(
                                                        common::LobbyUpdateNotification { lobby: Some(details) }
                                                    )),
                                                })).await;
                                            }
                                        }
                                        Err(e) => {
                                            let _ = tx.send(Ok(MenuServerMessage {
                                                message: Some(common::menu_server_message::Message::Error(
                                                    ErrorResponse { message: e }
                                                )),
                                            })).await;
                                        }
                                    }
                                }
                                common::menu_client_message::Message::MarkReady(req) => {
                                    if client_id.is_none() {
                                        continue;
                                    }

                                    match lobby_manager.mark_ready(&msg_client_id, req.ready).await {
                                        Ok(details) => {
                                            log!("{} marked ready: {}", msg_client_id, req.ready);
                                            let _ = tx.send(Ok(MenuServerMessage {
                                                message: Some(common::menu_server_message::Message::LobbyUpdate(
                                                    common::LobbyUpdateNotification { lobby: Some(details) }
                                                )),
                                            })).await;
                                        }
                                        Err(e) => {
                                            let _ = tx.send(Ok(MenuServerMessage {
                                                message: Some(common::menu_server_message::Message::Error(
                                                    ErrorResponse { message: e }
                                                )),
                                            })).await;
                                        }
                                    }
                                }
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
                lobby_manager.cleanup_client(&id).await;
                log!("Menu client disconnected (stream ended): {}", id);
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
