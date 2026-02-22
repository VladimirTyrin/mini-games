use tokio::sync::mpsc;
use tonic::Status;

use crate::{
    client_message, log, server_message, ClientId, ClientMessage, ErrorCode, ErrorResponse,
    ServerMessage,
};

use crate::broadcaster::Broadcaster;
use crate::game_session_manager::GameSessionManager;
use crate::lobby::{BotType, LobbyManager, LobbyStateAfterLeave, PlayAgainStatus, LobbySettings};

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
        let server_version = crate::version::get_version();
        if client_message.version != server_version {
            let error_text = format!(
                "Version mismatch: client version '{}', server version '{}'",
                client_message.version, server_version
            );
            log!("[pre-auth] Error: {}", error_text);
            let error_msg = ServerMessage {
                message: Some(server_message::Message::Error(ErrorResponse {
                    code: ErrorCode::VersionMismatch.into(),
                    message: error_text,
                })),
            };
            send_via_tx(tx, error_msg).await;
            return HandleResult::Disconnect;
        }

        let Some(message) = client_message.message else {
            return HandleResult::Continue;
        };

        match message {
            client_message::Message::Connect(connect_req) => {
                if client_id_opt.is_some() {
                    log!("[pre-auth] Error: Already connected");
                    send_via_tx(tx, make_error_response("Already connected".to_string())).await;
                    return HandleResult::Continue;
                }

                let client_id = ClientId::new(connect_req.client_id);

                if !self.lobby_manager.add_client(&client_id).await {
                    let error_text = "Client ID already connected. Only one connection per client ID is allowed.";
                    log!("[connect:{}] Error: {}", client_id, error_text);
                    let response = ServerMessage {
                        message: Some(server_message::Message::Connect(crate::ConnectResponse {
                            success: false,
                            error_message: error_text.to_string(),
                        })),
                    };
                    send_via_tx(tx, response).await;
                    return HandleResult::Disconnect;
                }

                self.broadcaster.register(client_id.clone(), tx.clone()).await;
                *client_id_opt = Some(client_id.clone());
                log!("Client connected: {}", client_id);

                let response = ServerMessage {
                    message: Some(server_message::Message::Connect(crate::ConnectResponse {
                        success: true,
                        error_message: String::new(),
                    })),
                };
                self.broadcaster.send_to_client(&client_id, response).await;
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
                    self.handle_list_lobbies(client_id).await;
                } else {
                    send_not_connected_error(tx, "list lobbies").await;
                }
            }
            client_message::Message::CreateLobby(req) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_create_lobby(client_id, req).await;
                } else {
                    send_not_connected_error(tx, "create lobby").await;
                }
            }
            client_message::Message::JoinLobby(req) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_join_lobby(client_id, req).await;
                } else {
                    send_not_connected_error(tx, "join lobby").await;
                }
            }
            client_message::Message::LeaveLobby(_) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_leave_lobby(client_id).await;
                } else {
                    send_not_connected_error(tx, "leave lobby").await;
                }
            }
            client_message::Message::MarkReady(req) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_mark_ready(client_id, req).await;
                } else {
                    send_not_connected_error(tx, "mark ready").await;
                }
            }
            client_message::Message::StartGame(_) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_start_game(client_id).await;
                } else {
                    send_not_connected_error(tx, "start game").await;
                }
            }
            client_message::Message::PlayAgain(_) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_play_again(client_id).await;
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
                    self.handle_add_bot(client_id, req).await;
                } else {
                    send_not_connected_error(tx, "add bot").await;
                }
            }
            client_message::Message::KickFromLobby(req) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_kick_from_lobby(client_id, req).await;
                } else {
                    send_not_connected_error(tx, "kick from lobby").await;
                }
            }
            client_message::Message::Ping(req) => {
                if let Some(client_id) = client_id_opt {
                    let pong = ServerMessage {
                        message: Some(server_message::Message::Pong(crate::PongResponse {
                            ping_id: req.ping_id,
                            client_timestamp_ms: req.client_timestamp_ms,
                        })),
                    };
                    self.broadcaster.send_to_client(client_id, pong).await;
                }
                return HandleResult::Continue;
            }
            client_message::Message::LobbyListChat(req) => {
                if let Some(client_id) = client_id_opt {
                    let clients = self.lobby_manager.get_clients_not_in_lobbies().await;
                    self.broadcaster
                        .broadcast_to_clients(
                            &clients,
                            ServerMessage {
                                message: Some(server_message::Message::LobbyListChat(
                                    crate::LobbyListChatNotification {
                                        sender: Some(crate::PlayerIdentity {
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
                                        crate::InLobbyChatNotification {
                                            sender: Some(crate::PlayerIdentity {
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
                    self.handle_become_observer(client_id).await;
                } else {
                    send_not_connected_error(tx, "become observer").await;
                }
            }
            client_message::Message::BecomePlayer(_) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_become_player(client_id).await;
                } else {
                    send_not_connected_error(tx, "become player").await;
                }
            }
            client_message::Message::MakeObserver(req) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_make_player_observer(client_id, req).await;
                } else {
                    send_not_connected_error(tx, "make player observer").await;
                }
            }
            client_message::Message::InReplay(cmd) => {
                if let Some(client_id) = client_id_opt {
                    self.session_manager.handle_replay_command(client_id, cmd).await;
                } else {
                    send_not_connected_error(tx, "send replay command").await;
                }
            }
            client_message::Message::CreateReplayLobby(req) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_create_replay_lobby(client_id, req).await;
                } else {
                    send_not_connected_error(tx, "create replay lobby").await;
                }
            }
            client_message::Message::WatchReplayTogether(req) => {
                if let Some(client_id) = client_id_opt {
                    self.handle_watch_replay_together(client_id, req).await;
                } else {
                    send_not_connected_error(tx, "watch replay together").await;
                }
            }
        }

        if let Some(client_id) = client_id_opt {
            self.lobby_manager.update_client_activity(client_id).await;

            if let Some(lobby_details) = self.lobby_manager.get_client_lobby(client_id).await {
                let lobby_id = crate::LobbyId::new(lobby_details.lobby_id);
                self.lobby_manager.update_lobby_activity(&lobby_id).await;
            }
        }

        HandleResult::Continue
    }

    async fn send_error(&self, client_id: &ClientId, message: String) {
        log!("[client:{}] Error: {}", client_id, message);
        self.broadcaster
            .send_to_client(client_id, make_error_response(message))
            .await;
    }

    pub async fn handle_client_disconnected(&self, client_id: &ClientId) {
        if let Ok(leave_state) = self.lobby_manager.leave_lobby(client_id).await {
            self.broadcast_leave_lobby_result(client_id, leave_state)
                .await;
        }

        self.lobby_manager.remove_client(client_id).await;
        self.broadcaster.unregister(client_id).await;
        self.session_manager.handle_player_disconnect(client_id).await;
    }

    async fn notify_lobby_list_update(&self) {
        let clients_not_in_lobbies = self.lobby_manager.get_clients_not_in_lobbies().await;
        self.broadcaster
            .broadcast_to_clients(
                &clients_not_in_lobbies,
                ServerMessage {
                    message: Some(server_message::Message::LobbyListUpdate(
                        crate::LobbyListUpdateNotification {},
                    )),
                },
            )
            .await;
    }

    async fn handle_list_lobbies(&self, client_id: &ClientId) {
        let lobbies = self.lobby_manager.list_lobbies().await;
        let response = ServerMessage {
            message: Some(server_message::Message::LobbyList(crate::LobbyListResponse {
                lobbies,
            })),
        };
        self.broadcaster.send_to_client(client_id, response).await;
    }

    async fn handle_create_lobby(&self, client_id: &ClientId, request: crate::CreateLobbyRequest) {
        let settings = match LobbySettings::from_proto(
            request.settings.and_then(|s| s.settings),
        ) {
            Ok(s) => s,
            Err(e) => {
                self.send_error(client_id, e).await;
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
                        crate::LobbyUpdateNotification {
                            details: Some(lobby_details.clone()),
                        },
                    )),
                };
                self.broadcaster.send_to_client(client_id, response).await;
                self.notify_lobby_list_update().await;
            }
            Err(e) => {
                self.send_error(client_id, e).await;
            }
        }
    }

    async fn handle_join_lobby(&self, client_id: &ClientId, request: crate::JoinLobbyRequest) {
        let lobby_id = crate::LobbyId::new(request.lobby_id);

        match self
            .lobby_manager
            .join_lobby(lobby_id, client_id.clone(), request.join_as_observer)
            .await
        {
            Ok(lobby_details) => {
                let response = ServerMessage {
                    message: Some(server_message::Message::LobbyUpdate(
                        crate::LobbyUpdateNotification {
                            details: Some(lobby_details.clone()),
                        },
                    )),
                };
                self.broadcaster.send_to_client(client_id, response).await;

                self.notify_lobby_list_update().await;

                self.broadcaster
                    .broadcast_to_lobby_except(
                        &lobby_details,
                        ServerMessage {
                            message: Some(server_message::Message::PlayerJoined(
                                crate::PlayerJoinedNotification {
                                    player: Some(crate::PlayerIdentity {
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
                                crate::LobbyUpdateNotification {
                                    details: Some(lobby_details.clone()),
                                },
                            )),
                        },
                    )
                    .await;
            }
            Err(e) => {
                self.send_error(client_id, e).await;
            }
        }
    }

    async fn handle_leave_lobby(&self, client_id: &ClientId) {
        match self.lobby_manager.leave_lobby(client_id).await {
            Ok(leave_state) => {
                let response = ServerMessage {
                    message: Some(server_message::Message::LobbyList(crate::LobbyListResponse {
                        lobbies: self.lobby_manager.list_lobbies().await,
                    })),
                };
                self.broadcaster.send_to_client(client_id, response).await;

                self.broadcast_leave_lobby_result(client_id, leave_state)
                    .await;
            }
            Err(e) => {
                self.send_error(client_id, e).await;
            }
        }
    }

    async fn broadcast_leave_lobby_result(
        &self,
        client_id: &ClientId,
        leave_state: LobbyStateAfterLeave,
    ) {
        match leave_state {
            LobbyStateAfterLeave::HostLeft { kicked_players } => {
                self.broadcaster
                    .broadcast_to_clients(
                        &kicked_players,
                        ServerMessage {
                            message: Some(server_message::Message::LobbyClosed(
                                crate::LobbyClosedNotification {
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
                                crate::PlayerLeftNotification {
                                    player: Some(crate::PlayerIdentity {
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
                                crate::LobbyUpdateNotification {
                                    details: Some(updated_details.clone()),
                                },
                            )),
                        },
                    )
                    .await;

                self.broadcaster
                    .broadcast_to_lobby(
                        &updated_details,
                        ServerMessage {
                            message: Some(server_message::Message::PlayAgainStatus(
                                crate::PlayAgainStatusNotification {
                                    ready_players: vec![],
                                    pending_players: vec![],
                                    available: false,
                                },
                            )),
                        },
                    )
                    .await;

                self.notify_lobby_list_update().await;
            }
        }
    }

    async fn handle_mark_ready(&self, client_id: &ClientId, request: crate::MarkReadyRequest) {
        match self.lobby_manager.mark_ready(client_id, request.ready).await {
            Ok(lobby_details) => {
                self.broadcaster
                    .broadcast_to_lobby(
                        &lobby_details,
                        ServerMessage {
                            message: Some(server_message::Message::PlayerReady(
                                crate::PlayerReadyNotification {
                                    player: Some(crate::PlayerIdentity {
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
                                crate::LobbyUpdateNotification {
                                    details: Some(lobby_details.clone()),
                                },
                            )),
                        },
                    )
                    .await;
            }
            Err(e) => {
                self.send_error(client_id, e).await;
            }
        }
    }

    async fn handle_add_bot(&self, client_id: &ClientId, request: crate::AddBotRequest) {
        let bot_type = match BotType::from_proto(request.bot_type) {
            Ok(bt) => bt,
            Err(e) => {
                self.send_error(client_id, e).await;
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
                                crate::PlayerJoinedNotification {
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
                                crate::LobbyUpdateNotification {
                                    details: Some(lobby_details.clone()),
                                },
                            )),
                        },
                    )
                    .await;

                self.notify_lobby_list_update().await;
            }
            Err(e) => {
                self.send_error(client_id, e).await;
            }
        }
    }

    async fn handle_kick_from_lobby(&self, client_id: &ClientId, request: crate::KickFromLobbyRequest) {
        match self
            .lobby_manager
            .kick_from_lobby(client_id, request.player_id)
            .await
        {
            Ok((lobby_details, kicked_identity, is_bot)) => {
                if !is_bot {
                    let kicked_client_id = ClientId::new(kicked_identity.client_id());

                    let kick_msg = ServerMessage {
                        message: Some(server_message::Message::LobbyClosed(
                            crate::LobbyClosedNotification {
                                message: "You were kicked from the lobby".to_string(),
                            },
                        )),
                    };
                    self.broadcaster
                        .send_to_client(&kicked_client_id, kick_msg)
                        .await;

                    self.broadcaster.unregister(&kicked_client_id).await;
                }

                self.broadcaster
                    .broadcast_to_lobby(
                        &lobby_details,
                        ServerMessage {
                            message: Some(server_message::Message::PlayerLeft(
                                crate::PlayerLeftNotification {
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
                                crate::LobbyUpdateNotification {
                                    details: Some(lobby_details.clone()),
                                },
                            )),
                        },
                    )
                    .await;

                self.notify_lobby_list_update().await;
            }
            Err(e) => {
                self.send_error(client_id, e).await;
            }
        }
    }

    async fn handle_become_observer(&self, client_id: &ClientId) {
        match self.lobby_manager.become_observer(client_id).await {
            Ok(lobby_details) => {
                self.broadcaster
                    .broadcast_to_lobby(
                        &lobby_details,
                        ServerMessage {
                            message: Some(server_message::Message::PlayerBecameObserver(
                                crate::PlayerBecameObserverNotification {
                                    player: Some(crate::PlayerIdentity {
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
                                crate::LobbyUpdateNotification {
                                    details: Some(lobby_details.clone()),
                                },
                            )),
                        },
                    )
                    .await;
            }
            Err(e) => {
                self.send_error(client_id, e).await;
            }
        }
    }

    async fn handle_become_player(&self, client_id: &ClientId) {
        match self.lobby_manager.become_player(client_id).await {
            Ok(lobby_details) => {
                self.broadcaster
                    .broadcast_to_lobby(
                        &lobby_details,
                        ServerMessage {
                            message: Some(server_message::Message::ObserverBecamePlayer(
                                crate::ObserverBecamePlayerNotification {
                                    observer: Some(crate::PlayerIdentity {
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
                                crate::LobbyUpdateNotification {
                                    details: Some(lobby_details.clone()),
                                },
                            )),
                        },
                    )
                    .await;
            }
            Err(e) => {
                self.send_error(client_id, e).await;
            }
        }
    }

    async fn handle_make_player_observer(
        &self,
        client_id: &ClientId,
        request: crate::MakePlayerObserverRequest,
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
                                crate::PlayerBecameObserverNotification {
                                    player: Some(crate::PlayerIdentity {
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
                                crate::LobbyUpdateNotification {
                                    details: Some(lobby_details.clone()),
                                },
                            )),
                        },
                    )
                    .await;
            }
            Err(e) => {
                self.send_error(client_id, e).await;
            }
        }
    }

    async fn handle_create_replay_lobby(
        &self,
        client_id: &ClientId,
        req: crate::CreateReplayLobbyRequest,
    ) {
        let replay_bytes = req.replay_content;
        let host_only_control = req.host_only_control;

        if let Err(e) = self.lobby_manager.leave_lobby(client_id).await {
            log!("[client:{}] Note: leave before replay lobby: {}", client_id, e);
        }

        match self
            .session_manager
            .create_replay_session(
                &self.lobby_manager,
                &self.broadcaster,
                replay_bytes,
                client_id.clone(),
                host_only_control,
            )
            .await
        {
            Ok(()) => {
                self.notify_lobby_list_update().await;
            }
            Err(e) => {
                self.send_error(client_id, e).await;
            }
        }
    }

    async fn handle_watch_replay_together(
        &self,
        client_id: &ClientId,
        req: crate::WatchReplayTogetherRequest,
    ) {
        let replay_bytes = req.replay_content;
        let host_only_control = req.host_only_control;

        let lobby_details = match self.lobby_manager.get_client_lobby(client_id).await {
            Some(details) => details,
            None => {
                self.send_error(client_id, "Not in a lobby".to_string()).await;
                return;
            }
        };

        let host_id_str = lobby_details
            .creator
            .as_ref()
            .map(|c| c.player_id.clone())
            .unwrap_or_default();

        if client_id.to_string() != host_id_str {
            self.send_error(client_id, "Only the host can start watch together".to_string())
                .await;
            return;
        }

        let human_count = lobby_details
            .players
            .iter()
            .filter(|p| p.identity.as_ref().is_some_and(|i| !i.is_bot))
            .count()
            + lobby_details
                .observers
                .iter()
                .filter(|o| !o.is_bot)
                .count();

        if human_count < 2 {
            self.send_error(
                client_id,
                "Need at least 2 human players to watch together".to_string(),
            )
            .await;
            return;
        }

        let viewer_ids: Vec<ClientId> = lobby_details
            .players
            .iter()
            .filter_map(|p| p.identity.as_ref())
            .filter(|i| !i.is_bot)
            .map(|i| ClientId::new(i.player_id.clone()))
            .chain(
                lobby_details
                    .observers
                    .iter()
                    .filter(|o| !o.is_bot)
                    .map(|o| ClientId::new(o.player_id.clone())),
            )
            .collect();

        let old_lobby_id = crate::LobbyId::new(lobby_details.lobby_id.clone());

        match self
            .session_manager
            .create_replay_session_for_group(
                &self.lobby_manager,
                &self.broadcaster,
                replay_bytes,
                client_id.clone(),
                viewer_ids,
                host_only_control,
            )
            .await
        {
            Ok(()) => {
                // Switch to replay lobby only after replay session was created successfully.
                self.lobby_manager.delete_lobby(&old_lobby_id).await;
                self.notify_lobby_list_update().await;
            }
            Err(e) => {
                self.send_error(client_id, e).await;
            }
        }
    }

    async fn handle_start_game(&self, client_id: &ClientId) {
        match self.lobby_manager.start_game(client_id).await {
            Ok(lobby_id) => {
                let session_id = lobby_id.to_string();

                if let Some(lobby_details) = self.lobby_manager.get_lobby_details(&lobby_id).await {
                    self.broadcaster
                        .broadcast_to_lobby(
                            &lobby_details,
                            ServerMessage {
                                message: Some(server_message::Message::GameStarting(
                                    crate::GameStartingNotification {
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
                self.send_error(client_id, e).await;
            }
        }
    }

    async fn handle_play_again(&self, client_id: &ClientId) {
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
                            .map(|id| crate::PlayerIdentity {
                                player_id: id.clone(),
                                is_bot: false,
                            })
                            .collect();
                        let pending = pending_player_ids
                            .iter()
                            .map(|id| crate::PlayerIdentity {
                                player_id: id.clone(),
                                is_bot: false,
                            })
                            .collect();
                        (ready, pending, true)
                    }
                };

                let status_msg = ServerMessage {
                    message: Some(server_message::Message::PlayAgainStatus(
                        crate::PlayAgainStatusNotification {
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
                                            crate::GameStartingNotification {
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
                self.send_error(client_id, e).await;
            }
        }
    }
}

async fn send_via_tx(tx: &ClientSender, message: ServerMessage) {
    if let Err(e) = tx.send(Ok(message)).await {
        log!("Failed to send message via tx: {}", e);
    }
}

async fn send_not_connected_error(tx: &ClientSender, action: &str) {
    let error_msg = format!("Not connected: cannot {}", action);
    log!("[pre-auth] Error: {}", error_msg);
    send_via_tx(tx, make_error_response(error_msg)).await;
}

fn make_error_response(message: String) -> ServerMessage {
    ServerMessage {
        message: Some(server_message::Message::Error(ErrorResponse {
            code: ErrorCode::Unspecified.into(),
            message,
        })),
    }
}
