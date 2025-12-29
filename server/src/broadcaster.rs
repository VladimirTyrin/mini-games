use tokio::sync::{mpsc, Mutex};
use std::collections::HashMap;
use std::sync::Arc;
use tonic::Status;
use common::{ClientId, LobbyDetails, ServerMessage};

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
            let player_id = ClientId::new(player.client_id.clone());
            if let Some(sender) = clients.get(&player_id) {
                let _ = sender.send(Ok(message.clone())).await;
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
            let player_id = ClientId::new(player.client_id.clone());
            if &player_id != except {
                if let Some(sender) = clients.get(&player_id) {
                    let _ = sender.send(Ok(message.clone())).await;
                }
            }
        }
    }

    pub async fn broadcast_to_all_except(&self, message: ServerMessage, except: &ClientId) {
        let clients = self.clients.lock().await;
        for (client_id, sender) in clients.iter() {
            if client_id != except {
                let _ = sender.send(Ok(message.clone())).await;
            }
        }
    }

    pub async fn broadcast_to_all(&self, message: ServerMessage) {
        let clients = self.clients.lock().await;
        for (_, sender) in clients.iter() {
            let _ = sender.send(Ok(message.clone())).await;
        }
    }

    pub async fn broadcast_to_clients(&self, client_ids: &Vec<ClientId>, message: ServerMessage) {
        let clients = self.clients.lock().await;
        for client_id in client_ids {
            if let Some(sender) = clients.get(client_id) {
                let _ = sender.send(Ok(message.clone())).await;
            }
        }
    }
}
