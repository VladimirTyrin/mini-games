use std::collections::{HashMap, HashSet};
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
    pub play_again_votes: HashSet<ClientId>,
    /// Players who were in the lobby when the last game started.
    /// Used to determine if "play again" is available (only if all original players are still in lobby).
    pub original_game_players: HashSet<ClientId>,
}

#[derive(Debug)]
pub enum LobbyStateAfterLeave {
    LobbyStillActive { updated_details: LobbyDetails },
    LobbyEmpty,
    HostLeft { kicked_players: Vec<ClientId> },
}


#[derive(Debug)]
pub enum PlayAgainStatus {
    NotAvailable,
    Available {
        ready_player_ids: Vec<String>,
        pending_player_ids: Vec<String>,
    },
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
            play_again_votes: HashSet::new(),
            original_game_players: HashSet::new(),
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

    fn has_ever_started(&self) -> bool {
        !self.original_game_players.is_empty()
    }

    fn get_pending_for_play_again(&self) -> Vec<String> {
        self.original_game_players.iter()
            .filter(|id| !self.play_again_votes.contains(id))
            .map(|id| id.to_string())
            .collect()
    }
}

#[derive(Debug)]
struct LobbyManagerState {
    lobbies: HashMap<LobbyId, Lobby>,
    client_to_lobby: HashMap<ClientId, LobbyId>,
    clients_not_in_lobby: HashSet<ClientId>,
    next_lobby_id: u64,
}

#[derive(Debug, Clone)]
pub struct LobbyManager {
    state: Arc<Mutex<LobbyManagerState>>,
}

impl LobbyManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(LobbyManagerState {
                lobbies: HashMap::new(),
                client_to_lobby: HashMap::new(),
                clients_not_in_lobby: HashSet::new(),
                next_lobby_id: 1,
            })),
        }
    }

    pub async fn add_client(&self, client_id: &ClientId) -> bool {
        let mut state = self.state.lock().await;

        if state.client_to_lobby.contains_key(client_id) || state.clients_not_in_lobby.contains(client_id) {
            return false;
        }

        state.clients_not_in_lobby.insert(client_id.clone());
        true
    }

    pub async fn remove_client(&self, client_id: &ClientId) {
        let mut state = self.state.lock().await;
        state.clients_not_in_lobby.remove(client_id);
    }

    pub async fn get_clients_not_in_lobbies(&self) -> Vec<ClientId> {
        let state = self.state.lock().await;
        state.clients_not_in_lobby.iter().cloned().collect()
    }

    pub async fn create_lobby(&self, name: String, max_players: u32, settings: LobbySettings, creator_id: ClientId) -> Result<LobbyDetails, String> {
        if settings.field_width < 5 || settings.field_width > 30 {
            return Err("Field width must be between 5 and 30".to_string());
        }

        if settings.field_height < 5 || settings.field_height > 30 {
            return Err("Field height must be between 5 and 30".to_string());
        }

        let mut state = self.state.lock().await;

        if state.client_to_lobby.contains_key(&creator_id) {
            return Err("Already in a lobby".to_string());
        }

        let lobby_id = LobbyId::new(format!("lobby_{}", state.next_lobby_id));
        state.next_lobby_id += 1;

        let mut lobby = Lobby::new(lobby_id.clone(), name, creator_id.clone(), max_players, settings);
        lobby.add_player(creator_id.clone());
        lobby.set_ready(&creator_id, true);

        let details = lobby.to_details();

        state.lobbies.insert(lobby_id.clone(), lobby);
        state.client_to_lobby.insert(creator_id.clone(), lobby_id);
        state.clients_not_in_lobby.remove(&creator_id);

        Ok(details)
    }

    pub async fn list_lobbies(&self) -> Vec<LobbyInfo> {
        let state = self.state.lock().await;
        state.lobbies.values()
            .filter(|lobby| !lobby.has_ever_started())
            .map(|lobby| lobby.to_info())
            .collect()
    }

    pub async fn get_lobby_details(&self, lobby_id: &LobbyId) -> Option<LobbyDetails> {
        let state = self.state.lock().await;
        state.lobbies.get(lobby_id).map(|lobby| lobby.to_details())
    }

    pub async fn join_lobby(&self, lobby_id: LobbyId, client_id: ClientId) -> Result<LobbyDetails, String> {
        let mut state = self.state.lock().await;

        if state.client_to_lobby.contains_key(&client_id) {
            return Err("Already in a lobby".to_string());
        }

        let lobby = state.lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

        if lobby.has_ever_started() {
            return Err("Cannot join: Lobby no longer accepting new players".to_string());
        }

        if !lobby.add_player(client_id.clone()) {
            return Err("Lobby is full or already joined".to_string());
        }

        let lobby_details = lobby.to_details();

        state.client_to_lobby.insert(client_id.clone(), lobby_id);
        state.clients_not_in_lobby.remove(&client_id);

        Ok(lobby_details)
    }

    pub async fn leave_lobby(&self, client_id: &ClientId) -> Result<LobbyStateAfterLeave, String> {
        let mut state = self.state.lock().await;

        let lobby_id = state.client_to_lobby.remove(client_id).ok_or("Not in a lobby")?;

        let result = {
            let lobby = state.lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;
            let is_host = &lobby.creator_id == client_id;
            lobby.remove_player(client_id);

            if is_host {
                LobbyStateAfterLeave::HostLeft {
                    kicked_players: lobby.players.keys().cloned().collect(),
                }
            } else if lobby.players.is_empty() {
                LobbyStateAfterLeave::LobbyEmpty
            } else {
                LobbyStateAfterLeave::LobbyStillActive {
                    updated_details: lobby.to_details(),
                }
            }
        };

        state.clients_not_in_lobby.insert(client_id.clone());

        match &result {
            LobbyStateAfterLeave::HostLeft { kicked_players } => {
                for player in kicked_players {
                    state.client_to_lobby.remove(player);
                    state.clients_not_in_lobby.insert(player.clone());
                }
                state.lobbies.remove(&lobby_id);
            }
            LobbyStateAfterLeave::LobbyEmpty => {
                state.lobbies.remove(&lobby_id);
            }
            LobbyStateAfterLeave::LobbyStillActive { .. } => {}
        }

        Ok(result)
    }

    pub async fn mark_ready(&self, client_id: &ClientId, ready: bool) -> Result<LobbyDetails, String> {
        let mut state = self.state.lock().await;

        let lobby_id = state.client_to_lobby.get(client_id).cloned().ok_or("Not in a lobby")?;

        let lobby = state.lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

        if !lobby.set_ready(client_id, ready) {
            return Err("Player not in lobby".to_string());
        }

        Ok(lobby.to_details())
    }

    pub async fn start_game(&self, client_id: &ClientId) -> Result<LobbyId, String> {
        let mut state = self.state.lock().await;

        let lobby_id = state.client_to_lobby.get(client_id).cloned().ok_or("Not in a lobby")?;

        let lobby = state.lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

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
        lobby.play_again_votes.clear();

        lobby.original_game_players = lobby.players.keys().cloned().collect();

        Ok(lobby_id)
    }

    pub async fn end_game(&self, lobby_id: &LobbyId) -> Result<Vec<ClientId>, String> {
        let mut state = self.state.lock().await;

        let lobby = state.lobbies.get_mut(lobby_id).ok_or("Lobby not found")?;
        let player_ids: Vec<ClientId> = lobby.players.keys().cloned().collect();

        lobby.in_game = false;

        for ready in lobby.players.values_mut() {
            *ready = false;
        }

        Ok(player_ids)
    }

    pub async fn vote_play_again(&self, client_id: &ClientId) -> Result<(LobbyId, PlayAgainStatus), String> {
        let mut state = self.state.lock().await;
        let lobby_id = state.client_to_lobby.get(client_id).ok_or("Not in a lobby")?.clone();

        let lobby = state.lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

        if lobby.in_game {
            return Err("Game is still in progress".to_string());
        }

        if !lobby.original_game_players.contains(client_id) {
            return Err("Player was not in the original game".to_string());
        }

        if !lobby.players.contains_key(client_id) {
            return Err("Player is no longer in the lobby".to_string());
        }

        let play_again_available = lobby.players.len() == lobby.original_game_players.len();
        if !play_again_available {
            return Ok((lobby_id, PlayAgainStatus::NotAvailable));
        }

        lobby.play_again_votes.insert(client_id.clone());
        lobby.set_ready(client_id, true);

        let ready_player_ids: Vec<String> = lobby.play_again_votes.iter().map(|id| id.to_string()).collect();
        let pending_player_ids: Vec<String> = lobby.get_pending_for_play_again();

        Ok((lobby_id, PlayAgainStatus::Available {
            ready_player_ids,
            pending_player_ids
        }))
    }

    pub async fn get_play_again_status(&self, lobby_id: &LobbyId) -> Result<PlayAgainStatus, String> {
        let state = self.state.lock().await;
        let lobby = state.lobbies.get(lobby_id).ok_or("Lobby not found")?;

        let play_again_available = !lobby.original_game_players.is_empty()
            && lobby.players.len() == lobby.original_game_players.len();

        if !play_again_available {
            return Ok(PlayAgainStatus::NotAvailable);
        }

        let ready_player_ids: Vec<String> = lobby.play_again_votes.iter().map(|id| id.to_string()).collect();
        let pending_player_ids: Vec<String> = lobby.get_pending_for_play_again();

        Ok(PlayAgainStatus::Available {
            ready_player_ids,
            pending_player_ids
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_test_settings() -> LobbySettings {
        LobbySettings {
            field_width: 15,
            field_height: 15,
            wall_collision_mode: common::WallCollisionMode::WrapAround.into(),
            tick_interval_ms: 200,
        }
    }

    #[tokio::test]
    async fn test_create_lobby_new_lobby_details_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());

        let result = manager.create_lobby(
            "Test Lobby".to_string(),
            4,
            default_test_settings(),
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
            default_test_settings(),
            creator_id.clone(),
        ).await.unwrap();

        let result = manager.create_lobby(
            "Second Lobby".to_string(),
            4,
            default_test_settings(),
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
            default_test_settings(),
            ClientId::new("creator1".to_string()),
        ).await.unwrap();

        manager.create_lobby(
            "Lobby 2".to_string(),
            2,
            default_test_settings(),
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
            default_test_settings(),
            ClientId::new("creator1".to_string()),
        ).await.unwrap();

        manager.create_lobby(
            "Game Lobby".to_string(),
            1,
            default_test_settings(),
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
            default_test_settings(),
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
            default_test_settings(),
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
            default_test_settings(),
            creator_id.clone(),
        ).await.unwrap();

        let lobby_id = LobbyId::new(details.lobby_id);

        manager.start_game(&creator_id).await.unwrap();

        let result = manager.join_lobby(lobby_id, joiner_id).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cannot join: Lobby no longer accepting new players");
    }

    #[tokio::test]
    async fn test_join_lobby_full_error_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());

        let details = manager.create_lobby(
            "Test Lobby".to_string(),
            2,
            default_test_settings(),
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
            default_test_settings(),
            creator_id,
        ).await.unwrap();

        let lobby_id = LobbyId::new(details.lobby_id);
        manager.join_lobby(lobby_id, joiner_id.clone()).await.unwrap();

        let result = manager.leave_lobby(&joiner_id).await;

        assert!(result.is_ok());
        let leave_state = result.unwrap();
        assert!(matches!(leave_state, LobbyStateAfterLeave::LobbyStillActive { .. }));
    }

    #[tokio::test]
    async fn test_leave_lobby_after_host_left_error_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());
        let joiner_id = ClientId::new("joiner".to_string());

        let details = manager.create_lobby(
            "Test Lobby".to_string(),
            4,
            default_test_settings(),
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
            default_test_settings(),
            creator_id.clone(),
        ).await.unwrap();

        let lobby_id = LobbyId::new(details.lobby_id);
        manager.join_lobby(lobby_id, joiner_id).await.unwrap();

        let result = manager.leave_lobby(&creator_id).await;

        assert!(result.is_ok());
        let leave_state = result.unwrap();
        match leave_state {
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
            default_test_settings(),
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
            default_test_settings(),
            creator_id.clone(),
        ).await.unwrap();

        let result = manager.start_game(&creator_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_game_non_host_error_returned() {
        let manager = LobbyManager::new();
        let creator_id = ClientId::new("creator".to_string());
        let joiner_id = ClientId::new("joiner".to_string());

        let details = manager.create_lobby(
            "Test Lobby".to_string(),
            4,
            default_test_settings(),
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
            default_test_settings(),
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
            default_test_settings(),
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
