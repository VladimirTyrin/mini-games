use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct ConnectionTracker {
    menu_clients: Arc<Mutex<HashSet<String>>>,
    game_clients: Arc<Mutex<HashSet<String>>>,
}

impl ConnectionTracker {
    pub fn new() -> Self {
        Self {
            menu_clients: Arc::new(Mutex::new(HashSet::new())),
            game_clients: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn add_menu_client(&self, client_id: &str) -> bool {
        let mut clients = self.menu_clients.lock().await;
        clients.insert(client_id.to_string())
    }

    pub async fn remove_menu_client(&self, client_id: &str) {
        let mut clients = self.menu_clients.lock().await;
        clients.remove(client_id);
    }

    pub async fn add_game_client(&self, client_id: &str) -> bool {
        let mut clients = self.game_clients.lock().await;
        clients.insert(client_id.to_string())
    }

    pub async fn remove_game_client(&self, client_id: &str) {
        let mut clients = self.game_clients.lock().await;
        clients.remove(client_id);
    }
}
