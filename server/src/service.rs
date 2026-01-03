use tonic::{Request, Response, Status};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use common::{
    snake_game_service_server::SnakeGameService,
    ClientMessage, ServerMessage, client_message, server_message,
    ClientId,
    log,
};
use crate::lobby_manager::LobbyManager;
use crate::broadcaster::Broadcaster;
use crate::game_session_manager::GameSessionManager;

#[derive(Debug)]
pub struct SnakeGameServiceImpl {
    lobby_manager: LobbyManager,
    broadcaster: Broadcaster,
    session_manager: GameSessionManager,
}

impl SnakeGameServiceImpl {
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
impl SnakeGameService for SnakeGameServiceImpl {
    type GameStreamStream = ReceiverStream<Result<ServerMessage, Status>>;

    async fn game_stream(
        &self,
        request: Request<tonic::Streaming<ClientMessage>>,
    ) -> Result<Response<Self::GameStreamStream>, Status> {
        let mut in_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(128);

        let lobby_manager = self.lobby_manager.clone();
        let broadcaster = self.broadcaster.clone();
        let session_manager = self.session_manager.clone();

        tokio::spawn(async move {
            let mut client_id_opt: Option<ClientId> = None;

            while let Some(result) = in_stream.next().await {
                match result {
                    Ok(client_message) => {
                        if let Some(message) = client_message.message {
                            match message {
                                client_message::Message::Connect(connect_req) => {
                                    if client_id_opt.is_some() {
                                        let error_msg = ServerMessage {
                                            message: Some(server_message::Message::Error(common::ErrorResponse {
                                                message: "Already connected".to_string(),
                                            })),
                                        };
                                        let _ = tx.send(Ok(error_msg)).await;
                                        continue;
                                    }

                                    let client_id = ClientId::new(connect_req.client_id);

                                    if !lobby_manager.add_client(&client_id).await {
                                        let error_msg = ServerMessage {
                                            message: Some(server_message::Message::Error(common::ErrorResponse {
                                                message: "Client ID already connected".to_string(),
                                            })),
                                        };
                                        let _ = tx.send(Ok(error_msg)).await;
                                        break;
                                    }

                                    broadcaster.register(client_id.clone(), tx.clone()).await;
                                    client_id_opt = Some(client_id);
                                    log!("Client connected: {}", client_id_opt.as_ref().unwrap());
                                }
                                client_message::Message::Disconnect(_) => {
                                    if let Some(client_id) = &client_id_opt {
                                        log!("Client requested disconnect: {}", client_id);
                                        Self::handle_client_disconnected(
                                            &lobby_manager,
                                            &broadcaster,
                                            &session_manager,
                                            client_id
                                        ).await;
                                    }
                                    break;
                                }
                                client_message::Message::ListLobbies(_) => {
                                    if let Some(client_id) = &client_id_opt {
                                        Self::handle_list_lobbies(&lobby_manager, &tx, client_id).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "list lobbies").await;
                                    }
                                }
                                client_message::Message::CreateLobby(req) => {
                                    if let Some(client_id) = &client_id_opt {
                                        Self::handle_create_lobby(&lobby_manager, &broadcaster, &tx, client_id, req).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "create lobby").await;
                                    }
                                }
                                client_message::Message::JoinLobby(req) => {
                                    if let Some(client_id) = &client_id_opt {
                                        Self::handle_join_lobby(&lobby_manager, &broadcaster, &tx, client_id, req).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "join lobby").await;
                                    }
                                }
                                client_message::Message::LeaveLobby(_) => {
                                    if let Some(client_id) = &client_id_opt {
                                        Self::handle_leave_lobby(&lobby_manager, &broadcaster, &tx, client_id).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "leave lobby").await;
                                    }
                                }
                                client_message::Message::MarkReady(req) => {
                                    if let Some(client_id) = &client_id_opt {
                                        Self::handle_mark_ready(&lobby_manager, &broadcaster, &tx, client_id, req).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "mark ready").await;
                                    }
                                }
                                client_message::Message::StartGame(_) => {
                                    if let Some(client_id) = &client_id_opt {
                                        Self::handle_start_game(&lobby_manager, &broadcaster, &session_manager, &tx, client_id).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "start game").await;
                                    }
                                }
                                client_message::Message::PlayAgain(_) => {
                                    if let Some(client_id) = &client_id_opt {
                                        Self::handle_play_again(&lobby_manager, &broadcaster, &session_manager, &tx, client_id).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "play again").await;
                                    }
                                }
                                client_message::Message::Turn(turn_cmd) => {
                                    if let Some(client_id) = &client_id_opt {
                                        Self::handle_turn(&session_manager, client_id, turn_cmd).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "send turn").await;
                                    }
                                }
                                client_message::Message::AddBot(req) => {
                                    if let Some(client_id) = &client_id_opt {
                                        Self::handle_add_bot(&lobby_manager, &broadcaster, &tx, client_id, req).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "add bot").await;
                                    }
                                }
                                client_message::Message::KickFromLobby(req) => {
                                    if let Some(client_id) = &client_id_opt {
                                        Self::handle_kick_from_lobby(&lobby_manager, &broadcaster, &tx, client_id, req).await;
                                    } else {
                                        Self::send_not_connected_error(&tx, "kick from lobby").await;
                                    }
                                }
                                client_message::Message::Ping(req) => {
                                    let pong = ServerMessage {
                                        message: Some(server_message::Message::Pong(common::PongResponse {
                                            ping_id: req.ping_id,
                                            client_timestamp_ms: req.client_timestamp_ms,
                                        })),
                                    };
                                    let _ = tx.send(Ok(pong)).await;
                                }
                            }
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
                Self::handle_client_disconnected(
                    &lobby_manager,
                    &broadcaster,
                    &session_manager,
                    client_id
                ).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

impl SnakeGameServiceImpl {
    async fn send_not_connected_error(
        tx: &mpsc::Sender<Result<ServerMessage, Status>>,
        action: &str,
    ) {
        let error_msg = ServerMessage {
            message: Some(server_message::Message::Error(common::ErrorResponse {
                message: format!("Not connected: cannot {}", action),
            })),
        };
        let _ = tx.send(Ok(error_msg)).await;
    }

    async fn notify_lobby_list_update(
        lobby_manager: &LobbyManager,
        broadcaster: &Broadcaster,
    ) {
        let clients_not_in_lobbies = lobby_manager.get_clients_not_in_lobbies().await;

        broadcaster.broadcast_to_clients(
            &clients_not_in_lobbies,
            ServerMessage {
                message: Some(server_message::Message::LobbyListUpdate(common::LobbyListUpdateNotification {})),
            },
        ).await;
    }

    async fn handle_list_lobbies(
        lobby_manager: &LobbyManager,
        tx: &mpsc::Sender<Result<ServerMessage, Status>>,
        _client_id: &ClientId,
    ) {
        let lobbies = lobby_manager.list_lobbies().await;
        let response = ServerMessage {
            message: Some(server_message::Message::LobbyList(common::LobbyListResponse {
                lobbies,
            })),
        };
        let _ = tx.send(Ok(response)).await;
    }

    async fn handle_create_lobby(
        lobby_manager: &LobbyManager,
        broadcaster: &Broadcaster,
        tx: &mpsc::Sender<Result<ServerMessage, Status>>,
        client_id: &ClientId,
        request: common::CreateLobbyRequest,
    ) {
        let settings = request.settings.unwrap_or_default();

        match lobby_manager.create_lobby(
            request.lobby_name,
            request.max_players,
            settings,
            client_id.clone(),
        ).await {
            Ok(lobby_details) => {
                let response = ServerMessage {
                    message: Some(server_message::Message::LobbyUpdate(common::LobbyUpdateNotification {
                        lobby: Some(lobby_details.clone()),
                    })),
                };
                let _ = tx.send(Ok(response)).await;

                Self::notify_lobby_list_update(lobby_manager, broadcaster).await;
            }
            Err(e) => {
                let error_msg = ServerMessage {
                    message: Some(server_message::Message::Error(common::ErrorResponse {
                        message: e,
                    })),
                };
                let _ = tx.send(Ok(error_msg)).await;
            }
        }
    }

    async fn handle_join_lobby(
        lobby_manager: &LobbyManager,
        broadcaster: &Broadcaster,
        tx: &mpsc::Sender<Result<ServerMessage, Status>>,
        client_id: &ClientId,
        request: common::JoinLobbyRequest,
    ) {
        let lobby_id = common::LobbyId::new(request.lobby_id);

        match lobby_manager.join_lobby(lobby_id, client_id.clone()).await {
            Ok(lobby_details) => {
                let response = ServerMessage {
                    message: Some(server_message::Message::LobbyUpdate(common::LobbyUpdateNotification {
                        lobby: Some(lobby_details.clone()),
                    })),
                };
                let _ = tx.send(Ok(response)).await;

                Self::notify_lobby_list_update(lobby_manager, broadcaster).await;

                broadcaster.broadcast_to_lobby_except(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::PlayerJoined(common::PlayerJoinedNotification {
                            identity: Some(common::PlayerIdentity {
                                player_id: client_id.to_string(),
                                is_bot: false,
                                bot_type: common::BotType::Unspecified as i32,
                            }),
                        })),
                    },
                    client_id,
                ).await;

                broadcaster.broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::LobbyUpdate(common::LobbyUpdateNotification {
                            lobby: Some(lobby_details.clone()),
                        })),
                    },
                ).await;
            }
            Err(e) => {
                let error_msg = ServerMessage {
                    message: Some(server_message::Message::Error(common::ErrorResponse {
                        message: e,
                    })),
                };
                let _ = tx.send(Ok(error_msg)).await;
            }
        }
    }

    async fn handle_leave_lobby(
        lobby_manager: &LobbyManager,
        broadcaster: &Broadcaster,
        tx: &mpsc::Sender<Result<ServerMessage, Status>>,
        client_id: &ClientId,
    ) {
        match lobby_manager.leave_lobby(client_id).await {
            Ok(leave_state) => {
                use crate::lobby_manager::LobbyStateAfterLeave;

                let response = ServerMessage {
                    message: Some(server_message::Message::LobbyList(common::LobbyListResponse {
                        lobbies: lobby_manager.list_lobbies().await,
                    })),
                };
                let _ = tx.send(Ok(response)).await;

                match leave_state {
                    LobbyStateAfterLeave::HostLeft { kicked_players } => {
                        broadcaster.broadcast_to_clients(
                            &kicked_players,
                            ServerMessage {
                                message: Some(server_message::Message::LobbyClosed(common::LobbyClosedNotification {
                                    message: "Host left the lobby".to_string(),
                                })),
                            },
                        ).await;
                        Self::notify_lobby_list_update(lobby_manager, broadcaster).await;
                    }
                    LobbyStateAfterLeave::LobbyEmpty => {
                        Self::notify_lobby_list_update(lobby_manager, broadcaster).await;
                    }
                    LobbyStateAfterLeave::LobbyStillActive { updated_details } => {
                        broadcaster.broadcast_to_lobby(
                            &updated_details,
                            ServerMessage {
                                message: Some(server_message::Message::PlayerLeft(common::PlayerLeftNotification {
                                    identity: Some(common::PlayerIdentity {
                                        player_id: client_id.to_string(),
                                        is_bot: false,
                                        bot_type: common::BotType::Unspecified as i32,
                                    }),
                                })),
                            },
                        ).await;

                        broadcaster.broadcast_to_lobby(
                            &updated_details,
                            ServerMessage {
                                message: Some(server_message::Message::LobbyUpdate(common::LobbyUpdateNotification {
                                    lobby: Some(updated_details.clone()),
                                })),
                            },
                        ).await;

                        Self::notify_lobby_list_update(lobby_manager, broadcaster).await;
                    }
                }
            }
            Err(e) => {
                let error_msg = ServerMessage {
                    message: Some(server_message::Message::Error(common::ErrorResponse {
                        message: e,
                    })),
                };
                let _ = tx.send(Ok(error_msg)).await;
            }
        }
    }

    async fn handle_mark_ready(
        lobby_manager: &LobbyManager,
        broadcaster: &Broadcaster,
        tx: &mpsc::Sender<Result<ServerMessage, Status>>,
        client_id: &ClientId,
        request: common::MarkReadyRequest,
    ) {
        match lobby_manager.mark_ready(client_id, request.ready).await {
            Ok(lobby_details) => {
                broadcaster.broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::PlayerReady(common::PlayerReadyNotification {
                            identity: Some(common::PlayerIdentity {
                                player_id: client_id.to_string(),
                                is_bot: false,
                                bot_type: common::BotType::Unspecified as i32,
                            }),
                            ready: request.ready,
                        })),
                    },
                ).await;

                broadcaster.broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::LobbyUpdate(common::LobbyUpdateNotification {
                            lobby: Some(lobby_details.clone()),
                        })),
                    },
                ).await;
            }
            Err(e) => {
                let error_msg = ServerMessage {
                    message: Some(server_message::Message::Error(common::ErrorResponse {
                        message: e,
                    })),
                };
                let _ = tx.send(Ok(error_msg)).await;
            }
        }
    }

    async fn handle_add_bot(
        lobby_manager: &LobbyManager,
        broadcaster: &Broadcaster,
        tx: &mpsc::Sender<Result<ServerMessage, Status>>,
        client_id: &ClientId,
        request: common::AddBotRequest,
    ) {
        let bot_type = common::BotType::try_from(request.bot_type)
            .unwrap_or(common::BotType::Unspecified);

        match lobby_manager.add_bot(client_id, bot_type).await {
            Ok((lobby_details, bot_identity)) => {
                broadcaster.broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::PlayerJoined(common::PlayerJoinedNotification {
                            identity: Some(bot_identity.to_proto()),
                        })),
                    },
                ).await;

                broadcaster.broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::LobbyUpdate(common::LobbyUpdateNotification {
                            lobby: Some(lobby_details.clone()),
                        })),
                    },
                ).await;

                Self::notify_lobby_list_update(lobby_manager, broadcaster).await;
            }
            Err(e) => {
                let error_msg = ServerMessage {
                    message: Some(server_message::Message::Error(common::ErrorResponse {
                        message: e,
                    })),
                };
                let _ = tx.send(Ok(error_msg)).await;
            }
        }
    }

    async fn handle_kick_from_lobby(
        lobby_manager: &LobbyManager,
        broadcaster: &Broadcaster,
        tx: &mpsc::Sender<Result<ServerMessage, Status>>,
        client_id: &ClientId,
        request: common::KickFromLobbyRequest,
    ) {
        match lobby_manager.kick_from_lobby(client_id, request.player_id).await {
            Ok((lobby_details, kicked_identity, is_bot)) => {
                if !is_bot {
                    let kicked_client_id = ClientId::new(kicked_identity.client_id());
                    broadcaster.unregister(&kicked_client_id).await;

                    let kick_msg = ServerMessage {
                        message: Some(server_message::Message::LobbyClosed(common::LobbyClosedNotification {
                            message: "You were kicked from the lobby".to_string(),
                        })),
                    };
                    broadcaster.broadcast_to_clients(&vec![kicked_client_id], kick_msg).await;
                }

                broadcaster.broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::PlayerLeft(common::PlayerLeftNotification {
                            identity: Some(kicked_identity.to_proto()),
                        })),
                    },
                ).await;

                broadcaster.broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::LobbyUpdate(common::LobbyUpdateNotification {
                            lobby: Some(lobby_details.clone()),
                        })),
                    },
                ).await;

                Self::notify_lobby_list_update(lobby_manager, broadcaster).await;
            }
            Err(e) => {
                let error_msg = ServerMessage {
                    message: Some(server_message::Message::Error(common::ErrorResponse {
                        message: e,
                    })),
                };
                let _ = tx.send(Ok(error_msg)).await;
            }
        }
    }

    async fn handle_start_game(
        lobby_manager: &LobbyManager,
        broadcaster: &Broadcaster,
        session_manager: &GameSessionManager,
        tx: &mpsc::Sender<Result<ServerMessage, Status>>,
        client_id: &ClientId,
    ) {
        match lobby_manager.start_game(client_id).await {
            Ok(lobby_id) => {
                let session_id = lobby_id.to_string();

                if let Some(lobby_details) = lobby_manager.get_lobby_details(&lobby_id).await {
                    session_manager.create_session(session_id.clone(), lobby_details.clone()).await;

                    broadcaster.broadcast_to_lobby(
                        &lobby_details,
                        ServerMessage {
                            message: Some(server_message::Message::GameStarting(common::GameStartingNotification {
                                session_id,
                            })),
                        },
                    ).await;

                    Self::notify_lobby_list_update(lobby_manager, broadcaster).await;
                }
            }
            Err(e) => {
                let error_msg = ServerMessage {
                    message: Some(server_message::Message::Error(common::ErrorResponse {
                        message: e,
                    })),
                };
                let _ = tx.send(Ok(error_msg)).await;
            }
        }
    }

    async fn handle_play_again(
        lobby_manager: &LobbyManager,
        broadcaster: &Broadcaster,
        session_manager: &GameSessionManager,
        tx: &mpsc::Sender<Result<ServerMessage, Status>>,
        client_id: &ClientId,
    ) {
        match lobby_manager.vote_play_again(client_id).await {
            Ok((lobby_id, status)) => {
                let lobby_details = match lobby_manager.get_lobby_details(&lobby_id).await {
                    Some(details) => details,
                    None => return,
                };

                let proto_status = match &status {
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

                let status_msg = ServerMessage {
                    message: Some(server_message::Message::PlayAgainStatus(
                        common::PlayAgainStatusNotification {
                            status: Some(proto_status),
                        }
                    )),
                };

                broadcaster.broadcast_to_lobby(&lobby_details, status_msg).await;

                if let crate::lobby_manager::PlayAgainStatus::Available { ready_player_ids: _, pending_player_ids } = status {
                    if pending_player_ids.is_empty() {
                        let host_id = ClientId::new(lobby_details.creator.as_ref().unwrap().player_id.clone());
                        if let Ok(lobby_id) = lobby_manager.start_game(&host_id).await {
                            let session_id = lobby_id.to_string();

                            if let Some(updated_lobby_details) = lobby_manager.get_lobby_details(&lobby_id).await {
                                session_manager.create_session(session_id.clone(), updated_lobby_details.clone()).await;

                                broadcaster.broadcast_to_lobby(
                                    &updated_lobby_details,
                                    ServerMessage {
                                        message: Some(server_message::Message::GameStarting(common::GameStartingNotification {
                                            session_id,
                                        })),
                                    },
                                ).await;

                                Self::notify_lobby_list_update(lobby_manager, broadcaster).await;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = ServerMessage {
                    message: Some(server_message::Message::Error(common::ErrorResponse {
                        message: e,
                    })),
                };
                let _ = tx.send(Ok(error_msg)).await;
            }
        }
    }

    async fn handle_turn(
        session_manager: &GameSessionManager,
        client_id: &ClientId,
        turn_cmd: common::TurnCommand,
    ) {
        use crate::game::Direction as GameDirection;

        let direction = match common::Direction::try_from(turn_cmd.direction) {
            Ok(common::Direction::Up) => GameDirection::Up,
            Ok(common::Direction::Down) => GameDirection::Down,
            Ok(common::Direction::Left) => GameDirection::Left,
            Ok(common::Direction::Right) => GameDirection::Right,
            _ => return,
        };

        session_manager.set_direction(client_id, direction).await;
    }

    async fn handle_client_disconnected(
        lobby_manager: &LobbyManager,
        broadcaster: &Broadcaster,
        session_manager: &GameSessionManager,
        client_id: &ClientId,
    ) {
        lobby_manager.remove_client(client_id).await;
        broadcaster.unregister(client_id).await;

        session_manager.kill_snake(client_id, crate::game::DeathReason::PlayerDisconnected).await;

        if let Ok(leave_state) = lobby_manager.leave_lobby(client_id).await {
            use crate::lobby_manager::LobbyStateAfterLeave;

            match leave_state {
                LobbyStateAfterLeave::HostLeft { kicked_players } => {
                    broadcaster.broadcast_to_clients(
                        &kicked_players,
                        ServerMessage {
                            message: Some(server_message::Message::LobbyClosed(common::LobbyClosedNotification {
                                message: "Host left the lobby".to_string(),
                            })),
                        },
                    ).await;
                    Self::notify_lobby_list_update(lobby_manager, broadcaster).await;
                }
                LobbyStateAfterLeave::LobbyEmpty => {
                    Self::notify_lobby_list_update(lobby_manager, broadcaster).await;
                }
                LobbyStateAfterLeave::LobbyStillActive { updated_details } => {
                    broadcaster.broadcast_to_lobby(
                        &updated_details,
                        ServerMessage {
                            message: Some(server_message::Message::PlayerLeft(common::PlayerLeftNotification {
                                identity: Some(common::PlayerIdentity {
                                    player_id: client_id.to_string(),
                                    is_bot: false,
                                    bot_type: common::BotType::Unspecified as i32,
                                }),
                            })),
                        },
                    ).await;

                    broadcaster.broadcast_to_lobby(
                        &updated_details,
                        ServerMessage {
                            message: Some(server_message::Message::LobbyUpdate(common::LobbyUpdateNotification {
                                lobby: Some(updated_details.clone()),
                            })),
                        },
                    ).await;

                    Self::notify_lobby_list_update(lobby_manager, broadcaster).await;
                }
            }
        }
    }
}
