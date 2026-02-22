use tokio::sync::{mpsc, Mutex};
use std::collections::HashMap;
use std::sync::Arc;
use tonic::Status;
use crate::{ClientId, LobbyDetails, ServerMessage, server_message, GameStateUpdate, GameOverNotification, log};
use crate::games::GameBroadcaster;

pub type ClientSender = mpsc::Sender<Result<ServerMessage, Status>>;

#[derive(Clone)]
pub struct Broadcaster {
    clients: Arc<Mutex<HashMap<ClientId, ClientSender>>>,
}

impl std::fmt::Debug for Broadcaster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Broadcaster").finish()
    }
}

impl Default for Broadcaster {
    fn default() -> Self {
        Self::new()
    }
}

impl Broadcaster {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register(&self, client_id: ClientId, sender: ClientSender) {
        self.clients.lock().await.insert(client_id, sender);
    }

    pub async fn unregister(&self, client_id: &ClientId) {
        self.clients.lock().await.remove(client_id);
    }

    pub async fn broadcast_to_lobby(&self, lobby_details: &LobbyDetails, message: ServerMessage) {
        let clients = self.clients.lock().await;
        for player in &lobby_details.players {
            if let Some(identity) = &player.identity
                && !identity.is_bot
            {
                let client_id = ClientId::new(identity.player_id.clone());
                if let Some(sender) = clients.get(&client_id)
                    && let Err(e) = sender.send(Ok(message.clone())).await
                {
                    log!("[lobby:{}] Failed to send to client {}: {}", lobby_details.lobby_id, client_id, e);
                }
            }
        }
        for observer in &lobby_details.observers {
            let client_id = ClientId::new(observer.player_id.clone());
            if let Some(sender) = clients.get(&client_id)
                && let Err(e) = sender.send(Ok(message.clone())).await
            {
                log!("[lobby:{}] Failed to send to observer {}: {}", lobby_details.lobby_id, client_id, e);
            }
        }
    }

    pub async fn broadcast_to_lobby_except(
        &self,
        lobby_details: &LobbyDetails,
        message: ServerMessage,
        except: &ClientId,
    ) {
        let clients = self.clients.lock().await;
        for player in &lobby_details.players {
            if let Some(identity) = &player.identity
                && !identity.is_bot
            {
                let client_id = ClientId::new(identity.player_id.clone());
                if &client_id != except
                    && let Some(sender) = clients.get(&client_id)
                    && let Err(e) = sender.send(Ok(message.clone())).await
                {
                    log!("[lobby:{}] Failed to send to client {}: {}", lobby_details.lobby_id, client_id, e);
                }
            }
        }
        for observer in &lobby_details.observers {
            let client_id = ClientId::new(observer.player_id.clone());
            if &client_id != except
                && let Some(sender) = clients.get(&client_id)
                && let Err(e) = sender.send(Ok(message.clone())).await
            {
                log!("[lobby:{}] Failed to send to observer {}: {}", lobby_details.lobby_id, client_id, e);
            }
        }
    }

    pub async fn broadcast_to_all(&self, message: ServerMessage) {
        let clients = self.clients.lock().await;
        for (client_id, sender) in clients.iter() {
            if let Err(e) = sender.send(Ok(message.clone())).await {
                log!("Failed to broadcast to client {}: {}", client_id, e);
            }
        }
    }

    pub async fn broadcast_to_clients(&self, client_ids: &[ClientId], message: ServerMessage) {
        let clients = self.clients.lock().await;
        for client_id in client_ids {
            if let Some(sender) = clients.get(client_id)
                && let Err(e) = sender.send(Ok(message.clone())).await
            {
                log!("Failed to send to client {}: {}", client_id, e);
            }
        }
    }

    pub async fn send_to_client(&self, client_id: &ClientId, message: ServerMessage) {
        let clients = self.clients.lock().await;
        if let Some(sender) = clients.get(client_id)
            && let Err(e) = sender.send(Ok(message)).await
        {
            log!("Failed to send to client {}: {}", client_id, e);
        }
    }
}

impl GameBroadcaster for Broadcaster {
    async fn broadcast_state(&self, state: GameStateUpdate, recipients: Vec<ClientId>) {
        let message = ServerMessage {
            message: Some(server_message::Message::GameState(state)),
        };
        self.broadcast_to_clients(&recipients, message).await;
    }

    async fn broadcast_game_over(&self, notification: GameOverNotification, recipients: Vec<ClientId>) {
        let message = ServerMessage {
            message: Some(server_message::Message::GameOver(notification)),
        };
        self.broadcast_to_clients(&recipients, message).await;
    }
}
