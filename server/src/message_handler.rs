use tokio::sync::mpsc;
use tonic::Status;

use common::{
    client_message, log, server_message, ClientId, ClientMessage, ErrorCode, ErrorResponse,
    ServerMessage,
};

use crate::broadcaster::Broadcaster;
use crate::game_session_manager::GameSessionManager;
use crate::lobby_manager::{BotType, LobbyManager, LobbyStateAfterLeave, PlayAgainStatus};

pub type ClientSender = mpsc::Sender<Result<ServerMessage, Status>>;

pub struct MessageHandler {
    lobby_manager: LobbyManager,
    broadcaster: Broadcaster,
    session_manager: GameSessionManager,
}

pub enum HandleResult {
    Continue,
    Disconnect,
}

impl MessageHandler {
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

    pub async fn handle_message(
        &self,
        client_message: ClientMessage,
        tx: &ClientSender,
        client_id_opt: &mut Option<ClientId>,
    ) -> HandleResult {
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
            send_to_client(tx, error_msg, client_id_opt.as_ref()).await;
            return HandleResult::Disconnect;
        }

        let Some(message) = client_message.message else {
            return HandleResult::Continue;
        };

        match message {
            client_message::Message::Connect(connect_req) => {
                if client_id_opt.is_some() {
                    send_to_client(
                        tx,
                        make_error_response("Already connected".to_string()),
                        client_id_opt.as_ref(),
                    )
                    .await;
                    return HandleResult::Continue;
                }

                let client_id = ClientId::new(connect_req.client_id);

                if !self.lobby_manager.add_client(&client_id).await {
                    send_to_client(
                        tx,
                        make_error_response("Client ID already connected".to_string()),
                        Some(&client_id),
                    )
                    .await;
                    return HandleResult::Disconnect;
                }

                self.broadcaster.register(client_id.clone(), tx.clone()).await;
                *client_id_opt = Some(client_id.clone());
                log!("Client connected: {}", client_id);
            }
            client_message::Message::Disconnect(_) => {
                if let Some(client_id) = client_id_opt {
                    log!("Client requested disconnect: {}", client_id);
                    self.handle_client_disconnected(client_id).await;
                }
                return HandleResult::Disconnect;
            }
            client_message::Message::ListLobbies(_) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_list_lobbies(tx, client_id).await;
                } else {
                    send_not_connected_error(tx, "list lobbies").await;
                }
            }
            client_message::Message::CreateLobby(req) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_create_lobby(tx, client_id, req).await;
                } else {
                    send_not_connected_error(tx, "create lobby").await;
                }
            }
            client_message::Message::JoinLobby(req) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_join_lobby(tx, client_id, req).await;
                } else {
                    send_not_connected_error(tx, "join lobby").await;
                }
            }
            client_message::Message::LeaveLobby(_) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_leave_lobby(tx, client_id).await;
                } else {
                    send_not_connected_error(tx, "leave lobby").await;
                }
            }
            client_message::Message::MarkReady(req) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_mark_ready(tx, client_id, req).await;
                } else {
                    send_not_connected_error(tx, "mark ready").await;
                }
            }
            client_message::Message::StartGame(_) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_start_game(tx, client_id).await;
                } else {
                    send_not_connected_error(tx, "start game").await;
                }
            }
            client_message::Message::PlayAgain(_) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_play_again(tx, client_id).await;
                } else {
                    send_not_connected_error(tx, "play again").await;
                }
            }
            client_message::Message::InGame(in_game_cmd) => {
                if let Some(client_id) = client_id_opt {
                    self.session_manager.handle_command(client_id, in_game_cmd).await;
                } else {
                    send_not_connected_error(tx, "send in-game command").await;
                }
            }
            client_message::Message::AddBot(req) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_add_bot(tx, client_id, req).await;
                } else {
                    send_not_connected_error(tx, "add bot").await;
                }
            }
            client_message::Message::KickFromLobby(req) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_kick_from_lobby(tx, client_id, req).await;
                } else {
                    send_not_connected_error(tx, "kick from lobby").await;
                }
            }
            client_message::Message::Ping(req) => {
                let pong = ServerMessage {
                    message: Some(server_message::Message::Pong(common::PongResponse {
                        ping_id: req.ping_id,
                        client_timestamp_ms: req.client_timestamp_ms,
                    })),
                };
                send_to_client(tx, pong, client_id_opt.as_ref()).await;
            }
            client_message::Message::LobbyListChat(req) => {
                if let Some(client_id) = client_id_opt {
                    let clients = self.lobby_manager.get_clients_not_in_lobbies().await;
                    self.broadcaster
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
                    send_not_connected_error(tx, "send lobby list chat message").await;
                }
            }
            client_message::Message::InLobbyChat(req) => {
                if let Some(client_id) = client_id_opt {
                    if let Some(lobby_details) = self.lobby_manager.get_client_lobby(client_id).await {
                        self.broadcaster
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
                        send_not_connected_error(tx, "send in-lobby chat message").await;
                    }
                } else {
                    send_not_connected_error(tx, "send in-lobby chat message").await;
                }
            }
            client_message::Message::BecomeObserver(_) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_become_observer(tx, client_id).await;
                } else {
                    send_not_connected_error(tx, "become observer").await;
                }
            }
            client_message::Message::BecomePlayer(_) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_become_player(tx, client_id).await;
                } else {
                    send_not_connected_error(tx, "become player").await;
                }
            }
            client_message::Message::MakeObserver(req) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_make_player_observer(tx, client_id, req).await;
                } else {
                    send_not_connected_error(tx, "make player observer").await;
                }
            }
        }

        HandleResult::Continue
    }

    pub async fn handle_client_disconnected(&self, client_id: &ClientId) {
        self.lobby_manager.remove_client(client_id).await;
        self.broadcaster.unregister(client_id).await;
        self.session_manager.handle_player_disconnect(client_id).await;

        if let Ok(leave_state) = self.lobby_manager.leave_lobby(client_id).await {
            match leave_state {
                LobbyStateAfterLeave::HostLeft { kicked_players } => {
                    self.broadcaster
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
                    self.notify_lobby_list_update().await;
                }
                LobbyStateAfterLeave::LobbyStillActive { updated_details } => {
                    self.broadcaster
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

                    self.broadcaster
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

                    self.notify_lobby_list_update().await;
                }
            }
        }
    }

    async fn notify_lobby_list_update(&self) {
        let clients_not_in_lobbies = self.lobby_manager.get_clients_not_in_lobbies().await;
        self.broadcaster
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

    async fn handle_list_lobbies(&self, tx: &ClientSender, client_id: &ClientId) {
        let lobbies = self.lobby_manager.list_lobbies().await;
        let response = ServerMessage {
            message: Some(server_message::Message::LobbyList(common::LobbyListResponse {
                lobbies,
            })),
        };
        send_to_client(tx, response, Some(client_id)).await;
    }

    async fn handle_create_lobby(
        &self,
        tx: &ClientSender,
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

        match self
            .lobby_manager
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
                self.notify_lobby_list_update().await;
            }
            Err(e) => {
                send_to_client(tx, make_error_response(e), Some(client_id)).await;
            }
        }
    }

    async fn handle_join_lobby(
        &self,
        tx: &ClientSender,
        client_id: &ClientId,
        request: common::JoinLobbyRequest,
    ) {
        let lobby_id = common::LobbyId::new(request.lobby_id);

        match self
            .lobby_manager
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

                self.notify_lobby_list_update().await;

                self.broadcaster
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

                self.broadcaster
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

    async fn handle_leave_lobby(&self, tx: &ClientSender, client_id: &ClientId) {
        match self.lobby_manager.leave_lobby(client_id).await {
            Ok(leave_state) => {
                let response = ServerMessage {
                    message: Some(server_message::Message::LobbyList(common::LobbyListResponse {
                        lobbies: self.lobby_manager.list_lobbies().await,
                    })),
                };
                send_to_client(tx, response, Some(client_id)).await;

                match leave_state {
                    LobbyStateAfterLeave::HostLeft { kicked_players } => {
                        self.broadcaster
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
                        self.notify_lobby_list_update().await;
                    }
                    LobbyStateAfterLeave::LobbyStillActive { updated_details } => {
                        self.broadcaster
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

                        self.broadcaster
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

                        self.notify_lobby_list_update().await;
                    }
                }
            }
            Err(e) => {
                send_to_client(tx, make_error_response(e), Some(client_id)).await;
            }
        }
    }

    async fn handle_mark_ready(
        &self,
        tx: &ClientSender,
        client_id: &ClientId,
        request: common::MarkReadyRequest,
    ) {
        match self.lobby_manager.mark_ready(client_id, request.ready).await {
            Ok(lobby_details) => {
                self.broadcaster
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

                self.broadcaster
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
        &self,
        tx: &ClientSender,
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

        match self.lobby_manager.add_bot(client_id, bot_type).await {
            Ok((lobby_details, bot_identity)) => {
                self.broadcaster
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

                self.broadcaster
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

                self.notify_lobby_list_update().await;
            }
            Err(e) => {
                send_to_client(tx, make_error_response(e), Some(client_id)).await;
            }
        }
    }

    async fn handle_kick_from_lobby(
        &self,
        tx: &ClientSender,
        client_id: &ClientId,
        request: common::KickFromLobbyRequest,
    ) {
        match self
            .lobby_manager
            .kick_from_lobby(client_id, request.player_id)
            .await
        {
            Ok((lobby_details, kicked_identity, is_bot)) => {
                if !is_bot {
                    let kicked_client_id = ClientId::new(kicked_identity.client_id());
                    self.broadcaster.unregister(&kicked_client_id).await;

                    let kick_msg = ServerMessage {
                        message: Some(server_message::Message::LobbyClosed(
                            common::LobbyClosedNotification {
                                message: "You were kicked from the lobby".to_string(),
                            },
                        )),
                    };
                    self.broadcaster
                        .broadcast_to_clients(&[kicked_client_id], kick_msg)
                        .await;
                }

                self.broadcaster
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

                self.broadcaster
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

                self.notify_lobby_list_update().await;
            }
            Err(e) => {
                send_to_client(tx, make_error_response(e), Some(client_id)).await;
            }
        }
    }

    async fn handle_become_observer(&self, tx: &ClientSender, client_id: &ClientId) {
        match self.lobby_manager.become_observer(client_id).await {
            Ok(lobby_details) => {
                self.broadcaster
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

                self.broadcaster
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

    async fn handle_become_player(&self, tx: &ClientSender, client_id: &ClientId) {
        match self.lobby_manager.become_player(client_id).await {
            Ok(lobby_details) => {
                self.broadcaster
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

                self.broadcaster
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
        &self,
        tx: &ClientSender,
        client_id: &ClientId,
        request: common::MakePlayerObserverRequest,
    ) {
        match self
            .lobby_manager
            .make_player_observer(client_id, request.player_id.clone())
            .await
        {
            Ok(lobby_details) => {
                self.broadcaster
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

                self.broadcaster
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

    async fn handle_start_game(&self, tx: &ClientSender, client_id: &ClientId) {
        match self.lobby_manager.start_game(client_id).await {
            Ok(lobby_id) => {
                let session_id = lobby_id.to_string();

                if let Some(lobby_details) = self.lobby_manager.get_lobby_details(&lobby_id).await {
                    self.broadcaster
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

                    self.session_manager
                        .create_session(session_id.clone(), lobby_details.clone())
                        .await;

                    self.notify_lobby_list_update().await;
                }
            }
            Err(e) => {
                send_to_client(tx, make_error_response(e), Some(client_id)).await;
            }
        }
    }

    async fn handle_play_again(&self, tx: &ClientSender, client_id: &ClientId) {
        match self.lobby_manager.vote_play_again(client_id).await {
            Ok((lobby_id, status)) => {
                let lobby_details = match self.lobby_manager.get_lobby_details(&lobby_id).await {
                    Some(details) => details,
                    None => return,
                };

                let (ready_players, pending_players, available) = match &status {
                    PlayAgainStatus::NotAvailable => (vec![], vec![], false),
                    PlayAgainStatus::Available {
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

                self.broadcaster
                    .broadcast_to_lobby(&lobby_details, status_msg)
                    .await;

                if let PlayAgainStatus::Available {
                    ready_player_ids: _,
                    pending_player_ids,
                } = status
                    && pending_player_ids.is_empty()
                {
                    let host_id =
                        ClientId::new(lobby_details.creator.as_ref().unwrap().player_id.clone());
                    if let Ok(lobby_id) = self.lobby_manager.start_game(&host_id).await {
                        let session_id = lobby_id.to_string();

                        if let Some(updated_lobby_details) =
                            self.lobby_manager.get_lobby_details(&lobby_id).await
                        {
                            self.broadcaster
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

                            self.session_manager
                                .create_session(session_id.clone(), updated_lobby_details.clone())
                                .await;

                            self.notify_lobby_list_update().await;
                        }
                    }
                }
            }
            Err(e) => {
                send_to_client(tx, make_error_response(e), Some(client_id)).await;
            }
        }
    }
}

pub async fn send_to_client(
    tx: &ClientSender,
    message: ServerMessage,
    client_id: Option<&ClientId>,
) {
    if let Err(e) = tx.send(Ok(message)).await {
        let client_str = client_id.map_or("unknown".to_string(), |id| id.to_string());
        log!("[client:{}] Failed to send message: {}", client_str, e);
    }
}

async fn send_not_connected_error(tx: &ClientSender, action: &str) {
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
