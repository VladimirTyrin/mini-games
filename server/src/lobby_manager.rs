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
}

pub struct LeaveLobbyDetails {
    pub state_after_leave: Option<LobbyDetails>,
    pub lobby_id: LobbyId
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

        let details = lobby.to_details();

        let mut lobbies = self.lobbies.lock().await;
        lobbies.insert(lobby_id.clone(), lobby);
        client_to_lobby.insert(creator_id, lobby_id);

        Ok(details)
    }

    pub async fn list_lobbies(&self) -> Vec<LobbyInfo> {
        let lobbies = self.lobbies.lock().await;
        lobbies.values().map(|lobby| lobby.to_info()).collect()
    }

    pub async fn join_lobby(&self, lobby_id: LobbyId, client_id: ClientId) -> Result<LobbyDetails, String> {
        let mut client_to_lobby = self.client_to_lobby.lock().await;

        if client_to_lobby.contains_key(&client_id) {
            return Err("Already in a lobby".to_string());
        }

        let mut lobbies = self.lobbies.lock().await;

        let lobby = lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

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
        lobby.remove_player(client_id);

        if lobby.players.is_empty() {
            lobbies.remove(&lobby_id);
            Ok(LeaveLobbyDetails {
                state_after_leave: None,
                lobby_id,
            })
        } else {
            Ok(LeaveLobbyDetails {
                state_after_leave: Some(
                    lobby.to_details()
                ),
                lobby_id,
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
}
