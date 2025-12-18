use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use common::ClientId;

#[derive(Debug, Clone)]
pub struct ConnectionTracker {
    menu_clients: Arc<Mutex<HashSet<ClientId>>>,
    game_clients: Arc<Mutex<HashSet<ClientId>>>,
}

impl ConnectionTracker {
    pub fn new() -> Self {
        Self {
            menu_clients: Arc::new(Mutex::new(HashSet::new())),
            game_clients: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn add_menu_client(&self, client_id: &ClientId) -> bool {
        let mut clients = self.menu_clients.lock().await;
        clients.insert(client_id.clone())
    }

    pub async fn remove_menu_client(&self, client_id: &ClientId) {
        let mut clients = self.menu_clients.lock().await;
        clients.remove(client_id);
    }

    pub async fn add_game_client(&self, client_id: &ClientId) -> bool {
        let mut clients = self.game_clients.lock().await;
        clients.insert(client_id.clone())
    }

    pub async fn remove_game_client(&self, client_id: &ClientId) {
        let mut clients = self.game_clients.lock().await;
        clients.remove(client_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_menu_client() {
        let tracker = ConnectionTracker::new();
        let result = tracker.add_menu_client(&ClientId::new("client1".to_string())).await;
        assert!(result);
    }

    #[tokio::test]
    async fn test_add_duplicate_menu_client() {
        let tracker = ConnectionTracker::new();
        let client_id = ClientId::new("client1".to_string());
        let result1 = tracker.add_menu_client(&client_id).await;
        let result2 = tracker.add_menu_client(&client_id).await;
        assert!(result1);
        assert!(!result2);
    }

    #[tokio::test]
    async fn test_remove_menu_client() {
        let tracker = ConnectionTracker::new();
        let client_id = ClientId::new("client1".to_string());
        tracker.add_menu_client(&client_id).await;
        tracker.remove_menu_client(&client_id).await;
        let result = tracker.add_menu_client(&client_id).await;
        assert!(result);
    }

    #[tokio::test]
    async fn test_add_game_client() {
        let tracker = ConnectionTracker::new();
        let result = tracker.add_game_client(&ClientId::new("client1".to_string())).await;
        assert!(result);
    }

    #[tokio::test]
    async fn test_add_duplicate_game_client() {
        let tracker = ConnectionTracker::new();
        let client_id = ClientId::new("client1".to_string());
        let result1 = tracker.add_game_client(&client_id).await;
        let result2 = tracker.add_game_client(&client_id).await;
        assert!(result1);
        assert!(!result2);
    }

    #[tokio::test]
    async fn test_remove_game_client() {
        let tracker = ConnectionTracker::new();
        let client_id = ClientId::new("client1".to_string());
        tracker.add_game_client(&client_id).await;
        tracker.remove_game_client(&client_id).await;
        let result = tracker.add_game_client(&client_id).await;
        assert!(result);
    }

    #[tokio::test]
    async fn test_menu_and_game_clients_independent() {
        let tracker = ConnectionTracker::new();
        let client_id = ClientId::new("client1".to_string());
        let menu_result = tracker.add_menu_client(&client_id).await;
        let game_result = tracker.add_game_client(&client_id).await;
        assert!(menu_result);
        assert!(game_result);
    }

    #[tokio::test]
    async fn test_multiple_clients() {
        let tracker = ConnectionTracker::new();
        let result1 = tracker.add_menu_client(&ClientId::new("client1".to_string())).await;
        let result2 = tracker.add_menu_client(&ClientId::new("client2".to_string())).await;
        let result3 = tracker.add_menu_client(&ClientId::new("client3".to_string())).await;
        assert!(result1);
        assert!(result2);
        assert!(result3);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_client() {
        let tracker = ConnectionTracker::new();
        let client_id = ClientId::new("client1".to_string());
        tracker.remove_menu_client(&client_id).await;
        let result = tracker.add_menu_client(&client_id).await;
        assert!(result);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let tracker = ConnectionTracker::new();
        let tracker_clone1 = tracker.clone();
        let tracker_clone2 = tracker.clone();

        let handle1 = tokio::spawn(async move {
            tracker_clone1.add_menu_client(&ClientId::new("client1".to_string())).await
        });

        let handle2 = tokio::spawn(async move {
            tracker_clone2.add_menu_client(&ClientId::new("client2".to_string())).await
        });

        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();

        assert!(result1);
        assert!(result2);
    }
}
