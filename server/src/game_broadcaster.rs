use tokio::sync::{mpsc, Mutex};
use std::collections::HashMap;
use std::sync::Arc;
use tonic::Status;
use common::{ClientId, GameServerMessage};

pub type GameClientSender = mpsc::Sender<Result<GameServerMessage, Status>>;

pub type SessionId = String;

#[derive(Clone)]
pub struct GameBroadcaster {
    sessions: Arc<Mutex<HashMap<SessionId, HashMap<ClientId, GameClientSender>>>>,
}

impl std::fmt::Debug for GameBroadcaster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameBroadcaster").finish()
    }
}

impl GameBroadcaster {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register(&self, session_id: SessionId, client_id: ClientId, sender: GameClientSender) {
        let mut sessions = self.sessions.lock().await;
        sessions.entry(session_id).or_insert_with(HashMap::new).insert(client_id, sender);
    }

    pub async fn unregister(&self, session_id: &SessionId, client_id: &ClientId) {
        let mut sessions = self.sessions.lock().await;
        if let Some(clients) = sessions.get_mut(session_id) {
            clients.remove(client_id);
            if clients.is_empty() {
                sessions.remove(session_id);
            }
        }
    }

    pub async fn broadcast_to_session(&self, session_id: &SessionId, message: GameServerMessage) {
        let sessions = self.sessions.lock().await;
        if let Some(clients) = sessions.get(session_id) {
            for (_, sender) in clients.iter() {
                let _ = sender.send(Ok(message.clone())).await;
            }
        }
    }
}
