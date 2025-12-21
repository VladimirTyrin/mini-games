use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use common::{LobbyInfo, LobbyDetails, PlayerInfo, LobbySettings, ClientId, LobbyId};

#[derive(Debug, Clone)]
pub struct Lobby {
    pub id: LobbyId,
    pub name: String,
    pub creator_id: ClientId,
    pub max_players: u32,
    pub settings: LobbySettings,
    pub players: HashMap<ClientId, bool>,
    pub in_game: bool,
}

#[derive(Debug)]
pub enum LobbyStateAfterLeave {
    LobbyStillActive { updated_details: LobbyDetails },
    LobbyEmpty,
    HostLeft { kicked_players: Vec<ClientId> },
}

#[derive(Debug)]
pub struct LeaveLobbyDetails {
    pub lobby_id: LobbyId,
    pub state: LobbyStateAfterLeave,
}

#[derive(Debug)]
pub struct StartGameResult {
    pub lobby_id: LobbyId,
    pub player_ids: Vec<ClientId>,
    pub settings: LobbySettings,
}

impl Lobby {
    fn new(id: LobbyId, name: String, creator_id: ClientId, max_players: u32, settings: LobbySettings) -> Self {
        Self {
            id,
            name,
            creator_id,
            max_players,
            settings,
            players: HashMap::new(),
            in_game: false,
        }
    }

    pub fn to_info(&self) -> LobbyInfo {
        LobbyInfo {
            lobby_id: self.id.to_string(),
            lobby_name: self.name.clone(),
            current_players: self.players.len() as u32,
            max_players: self.max_players,
        }
    }

    pub fn to_details(&self) -> LobbyDetails {
        let players: Vec<PlayerInfo> = self.players.iter().map(|(client_id, ready)| {
            PlayerInfo {
                client_id: client_id.to_string(),
                ready: *ready,
            }
        }).collect();

        LobbyDetails {
            lobby_id: self.id.to_string(),
            lobby_name: self.name.clone(),
            players,
            max_players: self.max_players,
            settings: Some(self.settings.clone()),
            creator_id: self.creator_id.to_string(),
        }
    }

    fn add_player(&mut self, client_id: ClientId) -> bool {
        if self.players.len() >= self.max_players as usize {
            return false;
        }
        if self.players.contains_key(&client_id) {
            return false;
        }
        self.players.insert(client_id, false);
        true
    }

    fn remove_player(&mut self, client_id: &ClientId) -> bool {
        self.players.remove(client_id).is_some()
    }

    fn set_ready(&mut self, client_id: &ClientId, ready: bool) -> bool {
        if let Some(player_ready) = self.players.get_mut(client_id) {
            *player_ready = ready;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct LobbyManager {
    lobbies: Arc<Mutex<HashMap<LobbyId, Lobby>>>,
    client_to_lobby: Arc<Mutex<HashMap<ClientId, LobbyId>>>,
    next_lobby_id: Arc<Mutex<u64>>,
}

impl LobbyManager {
    pub fn new() -> Self {
        Self {
            lobbies: Arc::new(Mutex::new(HashMap::new())),
            client_to_lobby: Arc::new(Mutex::new(HashMap::new())),
            next_lobby_id: Arc::new(Mutex::new(1)),
        }
    }

    pub async fn create_lobby(&self, name: String, max_players: u32, settings: LobbySettings, creator_id: ClientId) -> Result<LobbyDetails, String> {
        if settings.field_width < 5 || settings.field_width > 30 {
            return Err("Field width must be between 5 and 30".to_string());
        }

        if settings.field_height < 5 || settings.field_height > 30 {
            return Err("Field height must be between 5 and 30".to_string());
        }

        let mut client_to_lobby = self.client_to_lobby.lock().await;

        if client_to_lobby.contains_key(&creator_id) {
            return Err("Already in a lobby".to_string());
        }

        let mut next_id = self.next_lobby_id.lock().await;
        let lobby_id = LobbyId::new(format!("lobby_{}", *next_id));
        *next_id += 1;
        drop(next_id);

        let mut lobby = Lobby::new(lobby_id.clone(), name, creator_id.clone(), max_players, settings);
        lobby.add_player(creator_id.clone());
        lobby.set_ready(&creator_id, true);

        let details = lobby.to_details();

        let mut lobbies = self.lobbies.lock().await;
        lobbies.insert(lobby_id.clone(), lobby);
        client_to_lobby.insert(creator_id, lobby_id);

        Ok(details)
    }

    pub async fn list_lobbies(&self) -> Vec<LobbyInfo> {
        let lobbies = self.lobbies.lock().await;
        lobbies.values()
            .filter(|lobby| !lobby.in_game)
            .map(|lobby| lobby.to_info())
            .collect()
    }

    pub async fn join_lobby(&self, lobby_id: LobbyId, client_id: ClientId) -> Result<LobbyDetails, String> {
        let mut client_to_lobby = self.client_to_lobby.lock().await;

        if client_to_lobby.contains_key(&client_id) {
            return Err("Already in a lobby".to_string());
        }

        let mut lobbies = self.lobbies.lock().await;

        let lobby = lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

        if lobby.in_game {
            return Err("Cannot join: Game already started".to_string());
        }

        if !lobby.add_player(client_id.clone()) {
            return Err("Lobby is full or already joined".to_string());
        }

        client_to_lobby.insert(client_id, lobby_id);
        Ok(lobby.to_details())
    }

    pub async fn leave_lobby(&self, client_id: &ClientId) -> Result<LeaveLobbyDetails, String> {
        let mut client_to_lobby = self.client_to_lobby.lock().await;

        let lobby_id = client_to_lobby.remove(client_id).ok_or("Not in a lobby")?;

        let mut lobbies = self.lobbies.lock().await;

        let lobby = lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

        let is_host = &lobby.creator_id == client_id;

        lobby.remove_player(client_id);

        if is_host {
            let kicked_players: Vec<ClientId> = lobby.players.keys().cloned().collect();

            for player in &kicked_players {
                client_to_lobby.remove(player);
            }

            lobbies.remove(&lobby_id);

            Ok(LeaveLobbyDetails {
                lobby_id,
                state: LobbyStateAfterLeave::HostLeft { kicked_players },
            })
        } else if lobby.players.is_empty() {
            lobbies.remove(&lobby_id);
            Ok(LeaveLobbyDetails {
                lobby_id,
                state: LobbyStateAfterLeave::LobbyEmpty,
            })
        } else {
            Ok(LeaveLobbyDetails {
                lobby_id,
                state: LobbyStateAfterLeave::LobbyStillActive {
                    updated_details: lobby.to_details(),
                },
            })
        }
    }

    pub async fn mark_ready(&self, client_id: &ClientId, ready: bool) -> Result<LobbyDetails, String> {
        let client_to_lobby = self.client_to_lobby.lock().await;

        let lobby_id = client_to_lobby.get(client_id).ok_or("Not in a lobby")?;

        let mut lobbies = self.lobbies.lock().await;

        let lobby = lobbies.get_mut(lobby_id).ok_or("Lobby not found")?;

        if !lobby.set_ready(client_id, ready) {
            return Err("Player not in lobby".to_string());
        }

        Ok(lobby.to_details())
    }

    pub async fn start_game(&self, client_id: &ClientId) -> Result<StartGameResult, String> {
        let client_to_lobby = self.client_to_lobby.lock().await;

        let lobby_id = client_to_lobby.get(client_id).ok_or("Not in a lobby")?;

        let mut lobbies = self.lobbies.lock().await;

        let lobby = lobbies.get_mut(lobby_id).ok_or("Lobby not found")?;

        if &lobby.creator_id != client_id {
            return Err("Only the host can start the game".to_string());
        }

        if lobby.in_game {
            return Err("Game already started".to_string());
        }

        let all_ready = lobby.players.values().all(|ready| *ready);
        if !all_ready {
            return Err("Not all players are ready".to_string());
        }

        lobby.in_game = true;

        let player_ids: Vec<ClientId> = lobby.players.keys().cloned().collect();
        let settings = lobby.settings.clone();

        Ok(StartGameResult {
            lobby_id: lobby_id.clone(),
            player_ids,
            settings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_lobby_new_lobby_details_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());

        let result = manager.create_lobby(
            "Test Lobby".to_string(),
            4,
            LobbySettings {},
            creator_id.clone(),
        ).await;

        assert!(result.is_ok());
        let details = result.unwrap();
        assert_eq!(details.lobby_name, "Test Lobby");
        assert_eq!(details.max_players, 4);
        assert_eq!(details.players.len(), 1);
        assert_eq!(details.creator_id, creator_id.to_string());
        assert!(details.players[0].ready);
    }

    #[tokio::test]
    async fn test_create_lobby_already_in_lobby_error_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());

        manager.create_lobby(
            "First Lobby".to_string(),
            4,
            LobbySettings {},
            creator_id.clone(),
        ).await.unwrap();

        let result = manager.create_lobby(
            "Second Lobby".to_string(),
            4,
            LobbySettings {},
            creator_id,
        ).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Already in a lobby");
    }

    #[tokio::test]
    async fn test_list_lobbies_empty_empty_list_returned() {
        let manager = LobbyManager::new();
        let lobbies = manager.list_lobbies().await;
        assert_eq!(lobbies.len(), 0);
    }

    #[tokio::test]
    async fn test_list_lobbies_active_lobbies_lobbies_returned() {
        let manager = LobbyManager::new();

        manager.create_lobby(
            "Lobby 1".to_string(),
            4,
            LobbySettings {},
            ClientId::new("creator1".to_string()),
        ).await.unwrap();

        manager.create_lobby(
            "Lobby 2".to_string(),
            2,
            LobbySettings {},
            ClientId::new("creator2".to_string()),
        ).await.unwrap();

        let lobbies = manager.list_lobbies().await;
        assert_eq!(lobbies.len(), 2);
    }

    #[tokio::test]
    async fn test_list_lobbies_filters_in_game_only_active_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());

        manager.create_lobby(
            "Active Lobby".to_string(),
            4,
            LobbySettings {},
            ClientId::new("creator1".to_string()),
        ).await.unwrap();

        manager.create_lobby(
            "Game Lobby".to_string(),
            1,
            LobbySettings {},
            creator_id.clone(),
        ).await.unwrap();

        manager.start_game(&creator_id).await.unwrap();

        let lobbies = manager.list_lobbies().await;
        assert_eq!(lobbies.len(), 1);
        assert_eq!(lobbies[0].lobby_name, "Active Lobby");
    }

    #[tokio::test]
    async fn test_join_lobby_valid_lobby_details_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());
        let joiner_id = ClientId::new("joiner".to_string());

        let details = manager.create_lobby(
            "Test Lobby".to_string(),
            4,
            LobbySettings {},
            creator_id,
        ).await.unwrap();

        let lobby_id = LobbyId::new(details.lobby_id.clone());
        let result = manager.join_lobby(lobby_id, joiner_id).await;

        assert!(result.is_ok());
        let details = result.unwrap();
        assert_eq!(details.players.len(), 2);
    }

    #[tokio::test]
    async fn test_join_lobby_already_in_lobby_error_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());

        let details = manager.create_lobby(
            "Test Lobby".to_string(),
            4,
            LobbySettings {},
            creator_id.clone(),
        ).await.unwrap();

        let lobby_id = LobbyId::new(details.lobby_id);
        let result = manager.join_lobby(lobby_id, creator_id).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Already in a lobby");
    }

    #[tokio::test]
    async fn test_join_lobby_nonexistent_error_returned() {
        let manager = LobbyManager::new();
        let joiner_id = ClientId::new("joiner".to_string());

        let result = manager.join_lobby(
            LobbyId::new("nonexistent".to_string()),
            joiner_id,
        ).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Lobby not found");
    }

    #[tokio::test]
    async fn test_join_lobby_game_started_error_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());
        let joiner_id = ClientId::new("joiner".to_string());

        let details = manager.create_lobby(
            "Test Lobby".to_string(),
            4,
            LobbySettings {},
            creator_id.clone(),
        ).await.unwrap();

        let lobby_id = LobbyId::new(details.lobby_id);

        manager.start_game(&creator_id).await.unwrap();

        let result = manager.join_lobby(lobby_id, joiner_id).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cannot join: Game already started");
    }

    #[tokio::test]
    async fn test_join_lobby_full_error_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());

        let details = manager.create_lobby(
            "Test Lobby".to_string(),
            2,
            LobbySettings {},
            creator_id,
        ).await.unwrap();

        let lobby_id = LobbyId::new(details.lobby_id);

        manager.join_lobby(lobby_id.clone(), ClientId::new("player1".to_string())).await.unwrap();

        let result = manager.join_lobby(lobby_id, ClientId::new("player2".to_string())).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Lobby is full or already joined");
    }

    #[tokio::test]
    async fn test_leave_lobby_non_host_with_others_lobby_still_active() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());
        let joiner_id = ClientId::new("joiner".to_string());

        let details = manager.create_lobby(
            "Test Lobby".to_string(),
            4,
            LobbySettings {},
            creator_id,
        ).await.unwrap();

        let lobby_id = LobbyId::new(details.lobby_id);
        manager.join_lobby(lobby_id, joiner_id.clone()).await.unwrap();

        let result = manager.leave_lobby(&joiner_id).await;

        assert!(result.is_ok());
        let leave_details = result.unwrap();
        assert!(matches!(leave_details.state, LobbyStateAfterLeave::LobbyStillActive { .. }));
    }

    #[tokio::test]
    async fn test_leave_lobby_after_host_left_error_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());
        let joiner_id = ClientId::new("joiner".to_string());

        let details = manager.create_lobby(
            "Test Lobby".to_string(),
            4,
            LobbySettings {},
            creator_id.clone(),
        ).await.unwrap();

        let lobby_id = LobbyId::new(details.lobby_id);
        manager.join_lobby(lobby_id, joiner_id.clone()).await.unwrap();

        manager.leave_lobby(&creator_id).await.unwrap();
        let result = manager.leave_lobby(&joiner_id).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not in a lobby");
    }

    #[tokio::test]
    async fn test_leave_lobby_host_players_kicked() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());
        let joiner_id = ClientId::new("joiner".to_string());

        let details = manager.create_lobby(
            "Test Lobby".to_string(),
            4,
            LobbySettings {},
            creator_id.clone(),
        ).await.unwrap();

        let lobby_id = LobbyId::new(details.lobby_id);
        manager.join_lobby(lobby_id, joiner_id).await.unwrap();

        let result = manager.leave_lobby(&creator_id).await;

        assert!(result.is_ok());
        let leave_details = result.unwrap();
        match leave_details.state {
            LobbyStateAfterLeave::HostLeft { kicked_players } => {
                assert_eq!(kicked_players.len(), 1);
            },
            _ => panic!("Expected HostLeft state"),
        }
    }

    #[tokio::test]
    async fn test_leave_lobby_not_in_lobby_error_returned() {
        let manager = LobbyManager::new();
        let client_id = ClientId::new("client".to_string());

        let result = manager.leave_lobby(&client_id).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not in a lobby");
    }

    #[tokio::test]
    async fn test_mark_ready_in_lobby_details_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());
        let joiner_id = ClientId::new("joiner".to_string());

        let details = manager.create_lobby(
            "Test Lobby".to_string(),
            4,
            LobbySettings {},
            creator_id,
        ).await.unwrap();

        let lobby_id = LobbyId::new(details.lobby_id);
        manager.join_lobby(lobby_id, joiner_id.clone()).await.unwrap();

        let result = manager.mark_ready(&joiner_id, true).await;

        assert!(result.is_ok());
        let details = result.unwrap();
        assert!(details.players.iter().any(|p| p.client_id == joiner_id.to_string() && p.ready));
    }

    #[tokio::test]
    async fn test_mark_ready_not_in_lobby_error_returned() {
        let manager = LobbyManager::new();
        let client_id = ClientId::new("client".to_string());

        let result = manager.mark_ready(&client_id, true).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not in a lobby");
    }

    #[tokio::test]
    async fn test_start_game_host_all_ready_success() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());

        manager.create_lobby(
            "Test Lobby".to_string(),
            4,
            LobbySettings {},
            creator_id.clone(),
        ).await.unwrap();

        let result = manager.start_game(&creator_id).await;

        assert!(result.is_ok());
        let start_result = result.unwrap();
        assert_eq!(start_result.player_ids.len(), 1);
    }

    #[tokio::test]
    async fn test_start_game_non_host_error_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());
        let joiner_id = ClientId::new("joiner".to_string());

        let details = manager.create_lobby(
            "Test Lobby".to_string(),
            4,
            LobbySettings {},
            creator_id,
        ).await.unwrap();

        let lobby_id = LobbyId::new(details.lobby_id);
        manager.join_lobby(lobby_id, joiner_id.clone()).await.unwrap();
        manager.mark_ready(&joiner_id, true).await.unwrap();

        let result = manager.start_game(&joiner_id).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Only the host can start the game");
    }

    #[tokio::test]
    async fn test_start_game_not_all_ready_error_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());
        let joiner_id = ClientId::new("joiner".to_string());

        let details = manager.create_lobby(
            "Test Lobby".to_string(),
            4,
            LobbySettings {},
            creator_id.clone(),
        ).await.unwrap();

        let lobby_id = LobbyId::new(details.lobby_id);
        manager.join_lobby(lobby_id, joiner_id).await.unwrap();

        let result = manager.start_game(&creator_id).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not all players are ready");
    }

    #[tokio::test]
    async fn test_start_game_already_started_error_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());

        manager.create_lobby(
            "Test Lobby".to_string(),
            4,
            LobbySettings {},
            creator_id.clone(),
        ).await.unwrap();

        manager.start_game(&creator_id).await.unwrap();
        let result = manager.start_game(&creator_id).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Game already started");
    }

    #[tokio::test]
    async fn test_start_game_not_in_lobby_error_returned() {
        let manager = LobbyManager::new();
        let client_id = ClientId::new("client".to_string());

        let result = manager.start_game(&client_id).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not in a lobby");
    }
}
