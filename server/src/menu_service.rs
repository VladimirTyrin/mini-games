use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use common::{
    menu_service_server::MenuService,
    MenuClientMessage, MenuServerMessage, ErrorResponse,
    ClientId, LobbyId,
    PlayerJoinedNotification, PlayerLeftNotification, PlayerReadyNotification,
    LobbyUpdateNotification, LobbyListUpdateNotification, LobbyClosedNotification,
    GameStartingNotification,
    log,
};
use crate::connection_tracker::ConnectionTracker;
use crate::lobby_manager::{LobbyManager, LobbyStateAfterLeave};
use crate::broadcaster::ClientBroadcaster;
use crate::game_session_manager::GameSessionManager;
use crate::game::WallCollisionMode;

#[derive(Clone, Debug)]
struct MenuServiceDependencies {
    tracker: ConnectionTracker,
    lobby_manager: LobbyManager,
    broadcaster: ClientBroadcaster,
    session_manager: GameSessionManager,
}

#[derive(Debug)]
pub struct MenuServiceImpl {
    dependencies: MenuServiceDependencies
}

impl MenuServiceImpl {
    pub fn new(tracker: ConnectionTracker, lobby_manager: LobbyManager, broadcaster: ClientBroadcaster, session_manager: GameSessionManager) -> Self {
        Self {
            dependencies: MenuServiceDependencies {
                tracker,
                lobby_manager,
                broadcaster,
                session_manager,
            }
        }
    }
}

impl MenuServiceImpl {
    async fn notify_after_create_lobby(
        dependencies: &MenuServiceDependencies,
        client_id: &ClientId,
    ) {
        dependencies.broadcaster.broadcast_to_all_except(
            MenuServerMessage {
                message: Some(common::menu_server_message::Message::LobbyListUpdate(
                    LobbyListUpdateNotification {}
                )),
            },
            client_id,
        ).await;
    }

    async fn notify_after_join_lobby(
        dependencies: &MenuServiceDependencies,
        client_id: &ClientId,
        lobby_details: &common::LobbyDetails,
    ) {
        dependencies.broadcaster.broadcast_to_all_except(
            MenuServerMessage {
                message: Some(common::menu_server_message::Message::LobbyListUpdate(
                    LobbyListUpdateNotification {}
                )),
            },
            client_id,
        ).await;

        dependencies.broadcaster.broadcast_to_lobby_except(
            lobby_details,
            MenuServerMessage {
                message: Some(common::menu_server_message::Message::PlayerJoined(
                    PlayerJoinedNotification {
                        client_id: client_id.to_string(),
                    }
                )),
            },
            client_id,
        ).await;

        dependencies.broadcaster.broadcast_to_lobby(
            lobby_details,
            MenuServerMessage {
                message: Some(common::menu_server_message::Message::LobbyUpdate(
                    LobbyUpdateNotification { lobby: Some(lobby_details.clone()) }
                )),
            },
        ).await;
    }

    async fn notify_after_leave_lobby(
        dependencies: &MenuServiceDependencies,
        client_id: &ClientId,
        leave_details: &crate::lobby_manager::LeaveLobbyDetails,
    ) {
        dependencies.broadcaster.broadcast_to_all_except(
            MenuServerMessage {
                message: Some(common::menu_server_message::Message::LobbyListUpdate(
                    LobbyListUpdateNotification {}
                )),
            },
            client_id,
        ).await;

        match &leave_details.state {
            LobbyStateAfterLeave::LobbyStillActive { updated_details } => {
                dependencies.broadcaster.broadcast_to_lobby(
                    updated_details,
                    MenuServerMessage {
                        message: Some(common::menu_server_message::Message::PlayerLeft(
                            PlayerLeftNotification {
                                client_id: client_id.to_string(),
                            }
                        )),
                    },
                ).await;

                dependencies.broadcaster.broadcast_to_lobby(
                    updated_details,
                    MenuServerMessage {
                        message: Some(common::menu_server_message::Message::LobbyUpdate(
                            LobbyUpdateNotification { lobby: Some(updated_details.clone()) }
                        )),
                    },
                ).await;
            }
            LobbyStateAfterLeave::HostLeft { kicked_players } => {
                dependencies.broadcaster.broadcast_to_clients(
                    kicked_players,
                    MenuServerMessage {
                        message: Some(common::menu_server_message::Message::LobbyClosed(
                            LobbyClosedNotification {
                                message: "Lobby closed: Host left".to_string(),
                            }
                        )),
                    }
                ).await;
            }
            LobbyStateAfterLeave::LobbyEmpty => {}
        }
    }

    async fn notify_after_mark_ready(
        dependencies: &MenuServiceDependencies,
        client_id: &ClientId,
        lobby_details: &common::LobbyDetails,
        ready: bool,
    ) {
        dependencies.broadcaster.broadcast_to_lobby_except(
            lobby_details,
            MenuServerMessage {
                message: Some(common::menu_server_message::Message::PlayerReady(
                    PlayerReadyNotification {
                        client_id: client_id.to_string(),
                        ready,
                    }
                )),
            },
            client_id,
        ).await;

        dependencies.broadcaster.broadcast_to_lobby(
            lobby_details,
            MenuServerMessage {
                message: Some(common::menu_server_message::Message::LobbyUpdate(
                    LobbyUpdateNotification { lobby: Some(lobby_details.clone()) }
                )),
            },
        ).await;
    }

    async fn notify_after_start_game(
        dependencies: &MenuServiceDependencies,
        client_id: &ClientId,
        start_result: &crate::lobby_manager::StartGameResult,
    ) {
        let session_id = start_result.lobby_id.to_string();

        let wall_mode = match common::WallCollisionMode::try_from(start_result.settings.wall_collision_mode) {
            Ok(common::WallCollisionMode::Death) => WallCollisionMode::Death,
            Ok(common::WallCollisionMode::WrapAround) => WallCollisionMode::WrapAround,
            _ => WallCollisionMode::WrapAround,
        };

        if let Err(e) = dependencies.session_manager.create_session(
            session_id.clone(),
            start_result.player_ids.clone(),
            start_result.settings.field_width as usize,
            start_result.settings.field_height as usize,
            wall_mode,
            std::time::Duration::from_millis(start_result.settings.tick_interval_ms as u64),
        ).await {
            log!("Failed to create game session: {}", e);
            return;
        }

        dependencies.broadcaster.broadcast_to_clients(
            &start_result.player_ids,
            MenuServerMessage {
                message: Some(common::menu_server_message::Message::GameStarting(
                    GameStartingNotification {
                        session_id,
                    }
                )),
            },
        ).await;

        dependencies.broadcaster.broadcast_to_all_except(
            MenuServerMessage {
                message: Some(common::menu_server_message::Message::LobbyListUpdate(
                    LobbyListUpdateNotification {}
                )),
            },
            client_id,
        ).await;
    }

    async fn send_not_connected_error(
        tx: &tokio::sync::mpsc::Sender<Result<MenuServerMessage, Status>>,
        action: &str,
    ) {
        let _ = tx.send(Ok(MenuServerMessage {
            message: Some(common::menu_server_message::Message::Error(ErrorResponse {
                message: format!("Not connected: cannot {}", action),
            })),
        })).await;
    }

    async fn handle_connect_message(
        dependencies: &MenuServiceDependencies,
        client_id: &ClientId,
        tx: &tokio::sync::mpsc::Sender<Result<MenuServerMessage, Status>>,
    ) -> bool {
        if dependencies.tracker.add_menu_client(client_id).await {
            dependencies.broadcaster.register(client_id.clone(), tx.clone()).await;
            log!("Menu client connected: {}", client_id);
            true
        } else {
            let _ = tx.send(Ok(MenuServerMessage {
                message: Some(common::menu_server_message::Message::Error(ErrorResponse {
                    message: "Client ID already connected".to_string(),
                })),
            })).await;
            false
        }
    }

    async fn handle_list_lobbies_message(
        dependencies: &MenuServiceDependencies,
        tx: &tokio::sync::mpsc::Sender<Result<MenuServerMessage, Status>>,
    ) {
        let lobbies = dependencies.lobby_manager.list_lobbies().await;
        let _ = tx.send(Ok(MenuServerMessage {
            message: Some(common::menu_server_message::Message::LobbyList(
                common::LobbyListResponse { lobbies }
            )),
        })).await;
    }

    async fn handle_create_lobby_message(
        dependencies: &MenuServiceDependencies,
        client_id: &ClientId,
        request: common::CreateLobbyRequest,
        tx: &tokio::sync::mpsc::Sender<Result<MenuServerMessage, Status>>,
    ) {
        match dependencies.lobby_manager.create_lobby(
            request.lobby_name,
            request.max_players,
            request.settings.unwrap_or_default(),
            client_id.clone(),
        ).await {
            Ok(details) => {
                log!("[{}] Lobby created by {}", details.lobby_id, client_id);

                Self::notify_after_create_lobby(dependencies, client_id).await;

                let _ = tx.send(Ok(MenuServerMessage {
                    message: Some(common::menu_server_message::Message::LobbyUpdate(
                        LobbyUpdateNotification { lobby: Some(details) }
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

    async fn handle_join_lobby_message(
        dependencies: &MenuServiceDependencies,
        client_id: &ClientId,
        request: common::JoinLobbyRequest,
        tx: &tokio::sync::mpsc::Sender<Result<MenuServerMessage, Status>>,
    ) {
        match dependencies.lobby_manager.join_lobby(
            LobbyId::new(request.lobby_id),
            client_id.clone(),
        ).await {
            Ok(details) => {
                log!("[{}] {} joined lobby", details.lobby_id, client_id);
                Self::notify_after_join_lobby(dependencies, client_id, &details).await;
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

    async fn handle_leave_lobby_message(
        dependencies: &MenuServiceDependencies,
        client_id: &ClientId,
        tx: &tokio::sync::mpsc::Sender<Result<MenuServerMessage, Status>>,
    ) {
        match dependencies.lobby_manager.leave_lobby(client_id).await {
            Ok(leave_details) => {
                log!("[{}] {} left lobby", leave_details.lobby_id, client_id);

                Self::notify_after_leave_lobby(dependencies, client_id, &leave_details).await;

                let lobbies = dependencies.lobby_manager.list_lobbies().await;
                let _ = tx.send(Ok(MenuServerMessage {
                    message: Some(common::menu_server_message::Message::LobbyList(
                        common::LobbyListResponse { lobbies }
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

    async fn handle_mark_ready_message(
        dependencies: &MenuServiceDependencies,
        client_id: &ClientId,
        request: common::MarkReadyRequest,
        tx: &tokio::sync::mpsc::Sender<Result<MenuServerMessage, Status>>,
    ) {
        match dependencies.lobby_manager.mark_ready(client_id, request.ready).await {
            Ok(details) => {
                log!("[{}] {} marked ready: {}", details.lobby_id, client_id, request.ready);
                Self::notify_after_mark_ready(dependencies, client_id, &details, request.ready).await;
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

    async fn handle_start_game_message(
        dependencies: &MenuServiceDependencies,
        client_id: &ClientId,
        tx: &tokio::sync::mpsc::Sender<Result<MenuServerMessage, Status>>,
    ) {
        match dependencies.lobby_manager.start_game(client_id).await {
            Ok(start_result) => {
                log!("[{}] Game starting with {} players", start_result.lobby_id, start_result.player_ids.len());
                Self::notify_after_start_game(dependencies, client_id, &start_result).await;
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

    async fn handle_client_disconnected(dependencies: &MenuServiceDependencies, id: ClientId) {
        dependencies.tracker.remove_menu_client(&id).await;
        dependencies.broadcaster.unregister(&id).await;

        if let Some(leave_details) = dependencies.lobby_manager.leave_lobby(&id).await.ok() {
            Self::notify_after_leave_lobby(dependencies, &id, &leave_details).await;
        }
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
        let dependencies = self.dependencies.clone();

        tokio::spawn(async move {
            let mut client_id: Option<ClientId> = None;

            while let Some(result) = stream.next().await {
                match result {
                    Ok(msg) => {
                        if let Some(message) = msg.message {
                            match message {
                                common::menu_client_message::Message::Connect(req) => {
                                    if client_id.is_some() {
                                        let _ = tx.send(Ok(MenuServerMessage {
                                            message: Some(common::menu_server_message::Message::Error(ErrorResponse {
                                                message: "Already connected".to_string(),
                                            })),
                                        })).await;
                                        continue;
                                    }

                                    let new_client_id = ClientId::new(req.client_id);
                                    if Self::handle_connect_message(&dependencies, &new_client_id, &tx).await {
                                        client_id = Some(new_client_id);
                                    } else {
                                        break;
                                    }
                                }
                                common::menu_client_message::Message::Disconnect(_) => {
                                    if let Some(id) = &client_id {
                                        Self::handle_client_disconnected(&dependencies, id.clone()).await;
                                        log!("Menu client disconnected: {}", id);
                                        client_id = None;
                                    }
                                    break;
                                }
                                common::menu_client_message::Message::ListLobbies(_) => {
                                    if let Some(_) = &client_id {
                                        Self::handle_list_lobbies_message(&dependencies, &tx).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "list lobbies").await;
                                    }
                                }
                                common::menu_client_message::Message::CreateLobby(req) => {
                                    if let Some(id) = &client_id {
                                        Self::handle_create_lobby_message(&dependencies, id, req, &tx).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "create lobby").await;
                                    }
                                }
                                common::menu_client_message::Message::JoinLobby(req) => {
                                    if let Some(id) = &client_id {
                                        Self::handle_join_lobby_message(&dependencies, id, req, &tx).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "join lobby").await;
                                    }
                                }
                                common::menu_client_message::Message::LeaveLobby(_) => {
                                    if let Some(id) = &client_id {
                                        Self::handle_leave_lobby_message(&dependencies, id, &tx).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "leave lobby").await;
                                    }
                                }
                                common::menu_client_message::Message::MarkReady(req) => {
                                    if let Some(id) = &client_id {
                                        Self::handle_mark_ready_message(&dependencies, id, req, &tx).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "mark ready").await;
                                    }
                                }
                                common::menu_client_message::Message::StartGame(_) => {
                                    if let Some(id) = &client_id {
                                        Self::handle_start_game_message(&dependencies, id, &tx).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "start game").await;
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
                log!("Menu client disconnected (stream ended): {}", id);
                Self::handle_client_disconnected(&dependencies, id).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
