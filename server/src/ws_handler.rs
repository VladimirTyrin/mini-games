use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use tokio::sync::mpsc;
use tonic::Status;

use common::{
    client_message, log, server_message, ClientId, ClientMessage, ErrorCode, ErrorResponse,
    ServerMessage,
};

use crate::broadcaster::Broadcaster;
use crate::game_session_manager::GameSessionManager;
use crate::lobby_manager::{BotType, LobbyManager, LobbyStateAfterLeave};
use crate::web_server::WebServerState;

pub async fn handle_websocket(socket: WebSocket, state: WebServerState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    let (tx, mut rx) = mpsc::channel::<Result<ServerMessage, Status>>(128);

    let send_task = tokio::spawn(async move {
        while let Some(result) = rx.recv().await {
            if let Ok(msg) = result {
                let mut buf = Vec::new();
                if msg.encode(&mut buf).is_ok() {
                    if ws_sender.send(Message::Binary(buf.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    let lobby_manager = state.lobby_manager;
    let broadcaster = state.broadcaster;
    let session_manager = state.session_manager;

    let mut client_id_opt: Option<ClientId> = None;

    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(msg) => {
                let data = match msg {
                    Message::Binary(data) => data.to_vec(),
                    Message::Close(_) => break,
                    _ => continue,
                };

                let client_message = match ClientMessage::decode(data.as_slice()) {
                    Ok(m) => m,
                    Err(e) => {
                        log!("Failed to decode ClientMessage: {}", e);
                        continue;
                    }
                };

                let server_version = common::version::get_version();
                if client_message.version != server_version {
                    let error_msg = ServerMessage {
                        message: Some(server_message::Message::Error(ErrorResponse {
                            code: ErrorCode::VersionMismatch.into(),
                            message: format!(
                                "Version mismatch: client version '{}', server version '{}'",
                                client_message.version, server_version
                            ),
                        })),
                    };
                    send_to_client(&tx, error_msg, client_id_opt.as_ref()).await;
                    break;
                }

                if let Some(message) = client_message.message {
                    match message {
                        client_message::Message::Connect(connect_req) => {
                            if client_id_opt.is_some() {
                                send_to_client(
                                    &tx,
                                    make_error_response("Already connected".to_string()),
                                    client_id_opt.as_ref(),
                                )
                                .await;
                                continue;
                            }

                            let client_id = ClientId::new(connect_req.client_id);

                            if !lobby_manager.add_client(&client_id).await {
                                send_to_client(
                                    &tx,
                                    make_error_response("Client ID already connected".to_string()),
                                    Some(&client_id),
                                )
                                .await;
                                break;
                            }

                            broadcaster.register(client_id.clone(), tx.clone()).await;
                            client_id_opt = Some(client_id);
                            log!("WebSocket client connected: {}", client_id_opt.as_ref().unwrap());
                        }
                        client_message::Message::Disconnect(_) => {
                            if let Some(client_id) = &client_id_opt {
                                log!("WebSocket client requested disconnect: {}", client_id);
                                handle_client_disconnected(
                                    &lobby_manager,
                                    &broadcaster,
                                    &session_manager,
                                    client_id,
                                )
                                .await;
                            }
                            break;
                        }
                        client_message::Message::ListLobbies(_) => {
                            if let Some(client_id) = &client_id_opt {
                                handle_list_lobbies(&lobby_manager, &tx, client_id).await;
                            } else {
                                send_not_connected_error(&tx, "list lobbies").await;
                            }
                        }
                        client_message::Message::CreateLobby(req) => {
                            if let Some(client_id) = &client_id_opt {
                                handle_create_lobby(&lobby_manager, &broadcaster, &tx, client_id, req)
                                    .await;
                            } else {
                                send_not_connected_error(&tx, "create lobby").await;
                            }
                        }
                        client_message::Message::JoinLobby(req) => {
                            if let Some(client_id) = &client_id_opt {
                                handle_join_lobby(&lobby_manager, &broadcaster, &tx, client_id, req)
                                    .await;
                            } else {
                                send_not_connected_error(&tx, "join lobby").await;
                            }
                        }
                        client_message::Message::LeaveLobby(_) => {
                            if let Some(client_id) = &client_id_opt {
                                handle_leave_lobby(&lobby_manager, &broadcaster, &tx, client_id).await;
                            } else {
                                send_not_connected_error(&tx, "leave lobby").await;
                            }
                        }
                        client_message::Message::MarkReady(req) => {
                            if let Some(client_id) = &client_id_opt {
                                handle_mark_ready(&lobby_manager, &broadcaster, &tx, client_id, req)
                                    .await;
                            } else {
                                send_not_connected_error(&tx, "mark ready").await;
                            }
                        }
                        client_message::Message::StartGame(_) => {
                            if let Some(client_id) = &client_id_opt {
                                handle_start_game(
                                    &lobby_manager,
                                    &broadcaster,
                                    &session_manager,
                                    &tx,
                                    client_id,
                                )
                                .await;
                            } else {
                                send_not_connected_error(&tx, "start game").await;
                            }
                        }
                        client_message::Message::PlayAgain(_) => {
                            if let Some(client_id) = &client_id_opt {
                                handle_play_again(
                                    &lobby_manager,
                                    &broadcaster,
                                    &session_manager,
                                    &tx,
                                    client_id,
                                )
                                .await;
                            } else {
                                send_not_connected_error(&tx, "play again").await;
                            }
                        }
                        client_message::Message::InGame(in_game_cmd) => {
                            if let Some(client_id) = &client_id_opt {
                                handle_in_game_command(&session_manager, client_id, in_game_cmd).await;
                            } else {
                                send_not_connected_error(&tx, "send in-game command").await;
                            }
                        }
                        client_message::Message::AddBot(req) => {
                            if let Some(client_id) = &client_id_opt {
                                handle_add_bot(&lobby_manager, &broadcaster, &tx, client_id, req).await;
                            } else {
                                send_not_connected_error(&tx, "add bot").await;
                            }
                        }
                        client_message::Message::KickFromLobby(req) => {
                            if let Some(client_id) = &client_id_opt {
                                handle_kick_from_lobby(&lobby_manager, &broadcaster, &tx, client_id, req)
                                    .await;
                            } else {
                                send_not_connected_error(&tx, "kick from lobby").await;
                            }
                        }
                        client_message::Message::Ping(req) => {
                            let pong = ServerMessage {
                                message: Some(server_message::Message::Pong(common::PongResponse {
                                    ping_id: req.ping_id,
                                    client_timestamp_ms: req.client_timestamp_ms,
                                })),
                            };
                            send_to_client(&tx, pong, client_id_opt.as_ref()).await;
                        }
                        client_message::Message::LobbyListChat(req) => {
                            if let Some(client_id) = &client_id_opt {
                                let clients = lobby_manager.get_clients_not_in_lobbies().await;

                                broadcaster
                                    .broadcast_to_clients(
                                        &clients,
                                        ServerMessage {
                                            message: Some(server_message::Message::LobbyListChat(
                                                common::LobbyListChatNotification {
                                                    sender: Some(common::PlayerIdentity {
                                                        player_id: client_id.to_string(),
                                                        is_bot: false,
                                                    }),
                                                    message: req.message,
                                                },
                                            )),
                                        },
                                    )
                                    .await;
                            } else {
                                send_not_connected_error(&tx, "send lobby list chat message").await;
                            }
                        }
                        client_message::Message::InLobbyChat(req) => {
                            if let Some(client_id) = &client_id_opt {
                                if let Some(lobby_details) =
                                    lobby_manager.get_client_lobby(client_id).await
                                {
                                    broadcaster
                                        .broadcast_to_lobby(
                                            &lobby_details,
                                            ServerMessage {
                                                message: Some(server_message::Message::InLobbyChat(
                                                    common::InLobbyChatNotification {
                                                        sender: Some(common::PlayerIdentity {
                                                            player_id: client_id.to_string(),
                                                            is_bot: false,
                                                        }),
                                                        message: req.message,
                                                    },
                                                )),
                                            },
                                        )
                                        .await;
                                } else {
                                    send_not_connected_error(&tx, "send in-lobby chat message").await;
                                }
                            } else {
                                send_not_connected_error(&tx, "send in-lobby chat message").await;
                            }
                        }
                        client_message::Message::BecomeObserver(_) => {
                            if let Some(client_id) = &client_id_opt {
                                handle_become_observer(&lobby_manager, &broadcaster, &tx, client_id)
                                    .await;
                            } else {
                                send_not_connected_error(&tx, "become observer").await;
                            }
                        }
                        client_message::Message::BecomePlayer(_) => {
                            if let Some(client_id) = &client_id_opt {
                                handle_become_player(&lobby_manager, &broadcaster, &tx, client_id).await;
                            } else {
                                send_not_connected_error(&tx, "become player").await;
                            }
                        }
                        client_message::Message::MakeObserver(req) => {
                            if let Some(client_id) = &client_id_opt {
                                handle_make_player_observer(
                                    &lobby_manager,
                                    &broadcaster,
                                    &tx,
                                    client_id,
                                    req,
                                )
                                .await;
                            } else {
                                send_not_connected_error(&tx, "make player observer").await;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log!("WebSocket error: {}", e);
                break;
            }
        }
    }

    if let Some(client_id) = &client_id_opt {
        log!("WebSocket connection ended for client: {}", client_id);
        handle_client_disconnected(&lobby_manager, &broadcaster, &session_manager, client_id).await;
    }

    send_task.abort();
}

async fn send_to_client(
    tx: &mpsc::Sender<Result<ServerMessage, Status>>,
    message: ServerMessage,
    client_id: Option<&ClientId>,
) {
    if let Err(e) = tx.send(Ok(message)).await {
        let client_str = client_id.map_or("unknown".to_string(), |id| id.to_string());
        log!("[ws:{}] Failed to send message: {}", client_str, e);
    }
}

async fn send_not_connected_error(tx: &mpsc::Sender<Result<ServerMessage, Status>>, action: &str) {
    send_to_client(
        tx,
        make_error_response(format!("Not connected: cannot {}", action)),
        None,
    )
    .await;
}

fn make_error_response(message: String) -> ServerMessage {
    ServerMessage {
        message: Some(server_message::Message::Error(ErrorResponse {
            code: ErrorCode::Unspecified.into(),
            message,
        })),
    }
}

async fn notify_lobby_list_update(lobby_manager: &LobbyManager, broadcaster: &Broadcaster) {
    let clients_not_in_lobbies = lobby_manager.get_clients_not_in_lobbies().await;

    broadcaster
        .broadcast_to_clients(
            &clients_not_in_lobbies,
            ServerMessage {
                message: Some(server_message::Message::LobbyListUpdate(
                    common::LobbyListUpdateNotification {},
                )),
            },
        )
        .await;
}

async fn handle_list_lobbies(
    lobby_manager: &LobbyManager,
    tx: &mpsc::Sender<Result<ServerMessage, Status>>,
    client_id: &ClientId,
) {
    let lobbies = lobby_manager.list_lobbies().await;
    let response = ServerMessage {
        message: Some(server_message::Message::LobbyList(
            common::LobbyListResponse { lobbies },
        )),
    };
    send_to_client(tx, response, Some(client_id)).await;
}

async fn handle_create_lobby(
    lobby_manager: &LobbyManager,
    broadcaster: &Broadcaster,
    tx: &mpsc::Sender<Result<ServerMessage, Status>>,
    client_id: &ClientId,
    request: common::CreateLobbyRequest,
) {
    let settings = match crate::lobby_manager::LobbySettings::from_proto(
        request.settings.and_then(|s| s.settings),
    ) {
        Ok(s) => s,
        Err(e) => {
            send_to_client(tx, make_error_response(e), Some(client_id)).await;
            return;
        }
    };

    match lobby_manager
        .create_lobby(
            request.lobby_name,
            request.max_players,
            settings,
            client_id.clone(),
        )
        .await
    {
        Ok(lobby_details) => {
            let response = ServerMessage {
                message: Some(server_message::Message::LobbyUpdate(
                    common::LobbyUpdateNotification {
                        details: Some(lobby_details.clone()),
                    },
                )),
            };
            send_to_client(tx, response, Some(client_id)).await;

            notify_lobby_list_update(lobby_manager, broadcaster).await;
        }
        Err(e) => {
            send_to_client(tx, make_error_response(e), Some(client_id)).await;
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

    match lobby_manager
        .join_lobby(lobby_id, client_id.clone(), request.join_as_observer)
        .await
    {
        Ok(lobby_details) => {
            let response = ServerMessage {
                message: Some(server_message::Message::LobbyUpdate(
                    common::LobbyUpdateNotification {
                        details: Some(lobby_details.clone()),
                    },
                )),
            };
            send_to_client(tx, response, Some(client_id)).await;

            notify_lobby_list_update(lobby_manager, broadcaster).await;

            broadcaster
                .broadcast_to_lobby_except(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::PlayerJoined(
                            common::PlayerJoinedNotification {
                                player: Some(common::PlayerIdentity {
                                    player_id: client_id.to_string(),
                                    is_bot: false,
                                }),
                            },
                        )),
                    },
                    client_id,
                )
                .await;

            broadcaster
                .broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::LobbyUpdate(
                            common::LobbyUpdateNotification {
                                details: Some(lobby_details.clone()),
                            },
                        )),
                    },
                )
                .await;
        }
        Err(e) => {
            send_to_client(tx, make_error_response(e), Some(client_id)).await;
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
            let response = ServerMessage {
                message: Some(server_message::Message::LobbyList(
                    common::LobbyListResponse {
                        lobbies: lobby_manager.list_lobbies().await,
                    },
                )),
            };
            send_to_client(tx, response, Some(client_id)).await;

            match leave_state {
                LobbyStateAfterLeave::HostLeft { kicked_players } => {
                    broadcaster
                        .broadcast_to_clients(
                            &kicked_players,
                            ServerMessage {
                                message: Some(server_message::Message::LobbyClosed(
                                    common::LobbyClosedNotification {
                                        message: "Lobby closed".to_string(),
                                    },
                                )),
                            },
                        )
                        .await;
                    notify_lobby_list_update(lobby_manager, broadcaster).await;
                }
                LobbyStateAfterLeave::LobbyStillActive { updated_details } => {
                    broadcaster
                        .broadcast_to_lobby(
                            &updated_details,
                            ServerMessage {
                                message: Some(server_message::Message::PlayerLeft(
                                    common::PlayerLeftNotification {
                                        player: Some(common::PlayerIdentity {
                                            player_id: client_id.to_string(),
                                            is_bot: false,
                                        }),
                                    },
                                )),
                            },
                        )
                        .await;

                    broadcaster
                        .broadcast_to_lobby(
                            &updated_details,
                            ServerMessage {
                                message: Some(server_message::Message::LobbyUpdate(
                                    common::LobbyUpdateNotification {
                                        details: Some(updated_details.clone()),
                                    },
                                )),
                            },
                        )
                        .await;

                    notify_lobby_list_update(lobby_manager, broadcaster).await;
                }
            }
        }
        Err(e) => {
            send_to_client(tx, make_error_response(e), Some(client_id)).await;
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
            broadcaster
                .broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::PlayerReady(
                            common::PlayerReadyNotification {
                                player: Some(common::PlayerIdentity {
                                    player_id: client_id.to_string(),
                                    is_bot: false,
                                }),
                                ready: request.ready,
                            },
                        )),
                    },
                )
                .await;

            broadcaster
                .broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::LobbyUpdate(
                            common::LobbyUpdateNotification {
                                details: Some(lobby_details.clone()),
                            },
                        )),
                    },
                )
                .await;
        }
        Err(e) => {
            send_to_client(tx, make_error_response(e), Some(client_id)).await;
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
    let bot_type = match BotType::from_proto(request.bot_type) {
        Ok(bt) => bt,
        Err(e) => {
            send_to_client(tx, make_error_response(e), Some(client_id)).await;
            return;
        }
    };

    match lobby_manager.add_bot(client_id, bot_type).await {
        Ok((lobby_details, bot_identity)) => {
            broadcaster
                .broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::PlayerJoined(
                            common::PlayerJoinedNotification {
                                player: Some(bot_identity.to_proto()),
                            },
                        )),
                    },
                )
                .await;

            broadcaster
                .broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::LobbyUpdate(
                            common::LobbyUpdateNotification {
                                details: Some(lobby_details.clone()),
                            },
                        )),
                    },
                )
                .await;

            notify_lobby_list_update(lobby_manager, broadcaster).await;
        }
        Err(e) => {
            send_to_client(tx, make_error_response(e), Some(client_id)).await;
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
    match lobby_manager
        .kick_from_lobby(client_id, request.player_id)
        .await
    {
        Ok((lobby_details, kicked_identity, is_bot)) => {
            if !is_bot {
                let kicked_client_id = ClientId::new(kicked_identity.client_id());
                broadcaster.unregister(&kicked_client_id).await;

                let kick_msg = ServerMessage {
                    message: Some(server_message::Message::LobbyClosed(
                        common::LobbyClosedNotification {
                            message: "You were kicked from the lobby".to_string(),
                        },
                    )),
                };
                broadcaster
                    .broadcast_to_clients(&[kicked_client_id], kick_msg)
                    .await;
            }

            broadcaster
                .broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::PlayerLeft(
                            common::PlayerLeftNotification {
                                player: Some(kicked_identity.to_proto()),
                            },
                        )),
                    },
                )
                .await;

            broadcaster
                .broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::LobbyUpdate(
                            common::LobbyUpdateNotification {
                                details: Some(lobby_details.clone()),
                            },
                        )),
                    },
                )
                .await;

            notify_lobby_list_update(lobby_manager, broadcaster).await;
        }
        Err(e) => {
            send_to_client(tx, make_error_response(e), Some(client_id)).await;
        }
    }
}

async fn handle_become_observer(
    lobby_manager: &LobbyManager,
    broadcaster: &Broadcaster,
    tx: &mpsc::Sender<Result<ServerMessage, Status>>,
    client_id: &ClientId,
) {
    match lobby_manager.become_observer(client_id).await {
        Ok(lobby_details) => {
            broadcaster
                .broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::PlayerBecameObserver(
                            common::PlayerBecameObserverNotification {
                                player: Some(common::PlayerIdentity {
                                    player_id: client_id.to_string(),
                                    is_bot: false,
                                }),
                            },
                        )),
                    },
                )
                .await;

            broadcaster
                .broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::LobbyUpdate(
                            common::LobbyUpdateNotification {
                                details: Some(lobby_details.clone()),
                            },
                        )),
                    },
                )
                .await;
        }
        Err(e) => {
            send_to_client(tx, make_error_response(e), Some(client_id)).await;
        }
    }
}

async fn handle_become_player(
    lobby_manager: &LobbyManager,
    broadcaster: &Broadcaster,
    tx: &mpsc::Sender<Result<ServerMessage, Status>>,
    client_id: &ClientId,
) {
    match lobby_manager.become_player(client_id).await {
        Ok(lobby_details) => {
            broadcaster
                .broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::ObserverBecamePlayer(
                            common::ObserverBecamePlayerNotification {
                                observer: Some(common::PlayerIdentity {
                                    player_id: client_id.to_string(),
                                    is_bot: false,
                                }),
                            },
                        )),
                    },
                )
                .await;

            broadcaster
                .broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::LobbyUpdate(
                            common::LobbyUpdateNotification {
                                details: Some(lobby_details.clone()),
                            },
                        )),
                    },
                )
                .await;
        }
        Err(e) => {
            send_to_client(tx, make_error_response(e), Some(client_id)).await;
        }
    }
}

async fn handle_make_player_observer(
    lobby_manager: &LobbyManager,
    broadcaster: &Broadcaster,
    tx: &mpsc::Sender<Result<ServerMessage, Status>>,
    client_id: &ClientId,
    request: common::MakePlayerObserverRequest,
) {
    match lobby_manager
        .make_player_observer(client_id, request.player_id.clone())
        .await
    {
        Ok(lobby_details) => {
            broadcaster
                .broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::PlayerBecameObserver(
                            common::PlayerBecameObserverNotification {
                                player: Some(common::PlayerIdentity {
                                    player_id: request.player_id,
                                    is_bot: false,
                                }),
                            },
                        )),
                    },
                )
                .await;

            broadcaster
                .broadcast_to_lobby(
                    &lobby_details,
                    ServerMessage {
                        message: Some(server_message::Message::LobbyUpdate(
                            common::LobbyUpdateNotification {
                                details: Some(lobby_details.clone()),
                            },
                        )),
                    },
                )
                .await;
        }
        Err(e) => {
            send_to_client(tx, make_error_response(e), Some(client_id)).await;
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
                broadcaster
                    .broadcast_to_lobby(
                        &lobby_details,
                        ServerMessage {
                            message: Some(server_message::Message::GameStarting(
                                common::GameStartingNotification {
                                    session_id: session_id.clone(),
                                },
                            )),
                        },
                    )
                    .await;

                session_manager
                    .create_session(session_id.clone(), lobby_details.clone())
                    .await;

                notify_lobby_list_update(lobby_manager, broadcaster).await;
            }
        }
        Err(e) => {
            send_to_client(tx, make_error_response(e), Some(client_id)).await;
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

            let (ready_players, pending_players, available) = match &status {
                crate::lobby_manager::PlayAgainStatus::NotAvailable => (vec![], vec![], false),
                crate::lobby_manager::PlayAgainStatus::Available {
                    ready_player_ids,
                    pending_player_ids,
                } => {
                    let ready = ready_player_ids
                        .iter()
                        .map(|id| common::PlayerIdentity {
                            player_id: id.clone(),
                            is_bot: false,
                        })
                        .collect();
                    let pending = pending_player_ids
                        .iter()
                        .map(|id| common::PlayerIdentity {
                            player_id: id.clone(),
                            is_bot: false,
                        })
                        .collect();
                    (ready, pending, true)
                }
            };

            let status_msg = ServerMessage {
                message: Some(server_message::Message::PlayAgainStatus(
                    common::PlayAgainStatusNotification {
                        ready_players,
                        pending_players,
                        available,
                    },
                )),
            };

            broadcaster.broadcast_to_lobby(&lobby_details, status_msg).await;

            if let crate::lobby_manager::PlayAgainStatus::Available {
                ready_player_ids: _,
                pending_player_ids,
            } = status
            {
                if pending_player_ids.is_empty() {
                    let host_id =
                        ClientId::new(lobby_details.creator.as_ref().unwrap().player_id.clone());
                    if let Ok(lobby_id) = lobby_manager.start_game(&host_id).await {
                        let session_id = lobby_id.to_string();

                        if let Some(updated_lobby_details) =
                            lobby_manager.get_lobby_details(&lobby_id).await
                        {
                            broadcaster
                                .broadcast_to_lobby(
                                    &updated_lobby_details,
                                    ServerMessage {
                                        message: Some(server_message::Message::GameStarting(
                                            common::GameStartingNotification {
                                                session_id: session_id.clone(),
                                            },
                                        )),
                                    },
                                )
                                .await;

                            session_manager
                                .create_session(session_id.clone(), updated_lobby_details.clone())
                                .await;

                            notify_lobby_list_update(lobby_manager, broadcaster).await;
                        }
                    }
                }
            }
        }
        Err(e) => {
            send_to_client(tx, make_error_response(e), Some(client_id)).await;
        }
    }
}

async fn handle_in_game_command(
    session_manager: &GameSessionManager,
    client_id: &ClientId,
    in_game_cmd: common::InGameCommand,
) {
    session_manager.handle_command(client_id, in_game_cmd).await;
}

async fn handle_client_disconnected(
    lobby_manager: &LobbyManager,
    broadcaster: &Broadcaster,
    session_manager: &GameSessionManager,
    client_id: &ClientId,
) {
    lobby_manager.remove_client(client_id).await;
    broadcaster.unregister(client_id).await;

    session_manager.handle_player_disconnect(client_id).await;

    if let Ok(leave_state) = lobby_manager.leave_lobby(client_id).await {
        match leave_state {
            LobbyStateAfterLeave::HostLeft { kicked_players } => {
                broadcaster
                    .broadcast_to_clients(
                        &kicked_players,
                        ServerMessage {
                            message: Some(server_message::Message::LobbyClosed(
                                common::LobbyClosedNotification {
                                    message: "Lobby closed".to_string(),
                                },
                            )),
                        },
                    )
                    .await;
                notify_lobby_list_update(lobby_manager, broadcaster).await;
            }
            LobbyStateAfterLeave::LobbyStillActive { updated_details } => {
                broadcaster
                    .broadcast_to_lobby(
                        &updated_details,
                        ServerMessage {
                            message: Some(server_message::Message::PlayerLeft(
                                common::PlayerLeftNotification {
                                    player: Some(common::PlayerIdentity {
                                        player_id: client_id.to_string(),
                                        is_bot: false,
                                    }),
                                },
                            )),
                        },
                    )
                    .await;

                broadcaster
                    .broadcast_to_lobby(
                        &updated_details,
                        ServerMessage {
                            message: Some(server_message::Message::LobbyUpdate(
                                common::LobbyUpdateNotification {
                                    details: Some(updated_details.clone()),
                                },
                            )),
                        },
                    )
                    .await;

                notify_lobby_list_update(lobby_manager, broadcaster).await;
            }
        }
    }
}
