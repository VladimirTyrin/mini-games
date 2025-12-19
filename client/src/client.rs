use common::menu_service_client::MenuServiceClient;
use common::{MenuClientMessage, ConnectRequest, DisconnectRequest, ListLobbiesRequest, CreateLobbyRequest, JoinLobbyRequest, LeaveLobbyRequest, MarkReadyRequest, LobbySettings, log};
use tokio::sync::mpsc;
use crate::state::{ClientCommand, SharedState, AppState};

#[derive(Clone)]
pub struct LoggingSender<T> {
    inner: mpsc::Sender<T>,
}

impl<T> LoggingSender<T>
where
    T: std::fmt::Debug,
{
    pub fn new(inner: mpsc::Sender<T>) -> Self {
        Self { inner }
    }

    pub async fn send(&self, value: T) -> Result<(), mpsc::error::SendError<T>> {
        log!("Sending request: {:?}", value);
        self.inner.send(value).await
    }

    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
}

pub async fn grpc_client_task(
    client_id: String,
    server_address: String,
    shared_state: SharedState,
    mut command_rx: mpsc::UnboundedReceiver<ClientCommand>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut menu_client = MenuServiceClient::connect(server_address).await?;

    let (tx_raw, rx) = mpsc::channel(128);
    let tx = LoggingSender::new(tx_raw);

    let menu_stream = menu_client
        .menu_stream(tokio_stream::wrappers::ReceiverStream::new(rx))
        .await?;
    let mut response_stream = menu_stream.into_inner();

    tx.send(MenuClientMessage {
        client_id: client_id.clone(),
        message: Some(common::menu_client_message::Message::Connect(ConnectRequest {})),
    })
    .await?;

    tx.send(MenuClientMessage {
        client_id: client_id.clone(),
        message: Some(common::menu_client_message::Message::ListLobbies(ListLobbiesRequest {})),
    })
    .await?;

    loop {
        tokio::select! {
            Some(command) = command_rx.recv() => {
                let message = match command {
                    ClientCommand::ListLobbies => {
                        Some(common::menu_client_message::Message::ListLobbies(ListLobbiesRequest {}))
                    }
                    ClientCommand::CreateLobby { name, max_players } => {
                        Some(common::menu_client_message::Message::CreateLobby(CreateLobbyRequest {
                            lobby_name: name,
                            max_players,
                            settings: Some(LobbySettings {}),
                        }))
                    }
                    ClientCommand::JoinLobby { lobby_id } => {
                        Some(common::menu_client_message::Message::JoinLobby(JoinLobbyRequest {
                            lobby_id,
                        }))
                    }
                    ClientCommand::LeaveLobby => {
                        Some(common::menu_client_message::Message::LeaveLobby(LeaveLobbyRequest {}))
                    }
                    ClientCommand::MarkReady { ready } => {
                        Some(common::menu_client_message::Message::MarkReady(MarkReadyRequest {
                            ready,
                        }))
                    }
                    ClientCommand::Disconnect => {
                        let _ = tx.send(MenuClientMessage {
                            client_id: client_id.clone(),
                            message: Some(common::menu_client_message::Message::Disconnect(DisconnectRequest {})),
                        }).await;
                        break;
                    }
                };

                if let Some(msg) = message {
                    if tx.send(MenuClientMessage {
                        client_id: client_id.clone(),
                        message: Some(msg),
                    }).await.is_err() {
                        break;
                    }
                }
            }

            result = response_stream.message() => {
                match result {
                    Ok(Some(server_msg)) => {
                        if let Some(msg) = server_msg.message {
                            match msg {
                                common::menu_server_message::Message::LobbyList(list) => {
                                    shared_state.set_state(AppState::LobbyList {
                                        lobbies: list.lobbies,
                                    });
                                }
                                common::menu_server_message::Message::LobbyUpdate(update) => {
                                    if let Some(details) = update.lobby {
                                        let current_state = shared_state.get_state();
                                        match current_state {
                                            AppState::InLobby { event_log, .. } => {
                                                shared_state.set_state(AppState::InLobby {
                                                    details,
                                                    event_log,
                                                });
                                            }
                                            _ => {
                                                shared_state.set_state(AppState::InLobby {
                                                    details,
                                                    event_log: vec![],
                                                });
                                            }
                                        }
                                    }
                                }
                                common::menu_server_message::Message::PlayerJoined(notification) => {
                                    shared_state.add_event(format!("{} joined", notification.client_id));
                                }
                                common::menu_server_message::Message::PlayerLeft(notification) => {
                                    shared_state.add_event(format!("{} left", notification.client_id));
                                }
                                common::menu_server_message::Message::PlayerReady(notification) => {
                                    if notification.ready {
                                        shared_state.add_event(format!("{} is ready", notification.client_id));
                                    } else {
                                        shared_state.add_event(format!("{} is not ready anymore", notification.client_id));
                                    }
                                }
                                common::menu_server_message::Message::LobbyListUpdate(_) => {
                                    let current_state = shared_state.get_state();
                                    if matches!(current_state, AppState::LobbyList { .. }) {
                                        if tx.send(MenuClientMessage {
                                            client_id: client_id.clone(),
                                            message: Some(common::menu_client_message::Message::ListLobbies(
                                                ListLobbiesRequest {}
                                            )),
                                        }).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                                common::menu_server_message::Message::Error(err) => {
                                    shared_state.set_error(err.message);
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        shared_state.set_error(format!("Connection error: {}", e));
                        break;
                    }
                }
            }
        }
    }

    let _ = tx.send(MenuClientMessage {
        client_id: client_id.clone(),
        message: Some(common::menu_client_message::Message::Disconnect(DisconnectRequest {})),
    }).await;

    Ok(())
}
