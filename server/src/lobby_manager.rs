use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use common::{LobbyInfo, LobbyDetails, ClientId, LobbyId, PlayerId, BotId};
use common::id_generator::generate_client_id;

pub use common::lobby::{
    Lobby, LobbySettings, BotType, PlayerIdentity,
    LobbyStateAfterLeave, PlayAgainStatus,
};

#[derive(Debug)]
struct LobbyManagerState {
    lobbies: HashMap<LobbyId, Lobby>,
    client_to_lobby: HashMap<ClientId, LobbyId>,
    clients_not_in_lobby: HashSet<ClientId>,
    next_bot_id: u64,
    next_lobby_id: u64,
    last_client_activity: HashMap<ClientId, Instant>,
    last_lobby_activity: HashMap<LobbyId, Instant>,
}

#[derive(Debug, Clone)]
pub struct LobbyManager {
    state: Arc<Mutex<LobbyManagerState>>,
}

impl Default for LobbyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LobbyManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(LobbyManagerState {
                lobbies: HashMap::new(),
                client_to_lobby: HashMap::new(),
                clients_not_in_lobby: HashSet::new(),
                next_bot_id: 1,
                next_lobby_id: 1,
                last_client_activity: HashMap::new(),
                last_lobby_activity: HashMap::new(),
            })),
        }
    }

    pub async fn add_client(&self, client_id: &ClientId) -> bool {
        let mut state = self.state.lock().await;

        if state.client_to_lobby.contains_key(client_id) || state.clients_not_in_lobby.contains(client_id) {
            return false;
        }

        state.clients_not_in_lobby.insert(client_id.clone());
        state.last_client_activity.insert(client_id.clone(), Instant::now());
        true
    }

    pub async fn remove_client(&self, client_id: &ClientId) {
        let mut state = self.state.lock().await;
        state.clients_not_in_lobby.remove(client_id);
        state.last_client_activity.remove(client_id);
    }

    pub async fn get_clients_not_in_lobbies(&self) -> Vec<ClientId> {
        let state = self.state.lock().await;
        state.clients_not_in_lobby.iter().cloned().collect()
    }

    pub async fn update_client_activity(&self, client_id: &ClientId) {
        let mut state = self.state.lock().await;
        state.last_client_activity.insert(client_id.clone(), Instant::now());
    }

    pub async fn update_lobby_activity(&self, lobby_id: &LobbyId) {
        let mut state = self.state.lock().await;
        state.last_lobby_activity.insert(lobby_id.clone(), Instant::now());
    }

    pub async fn get_inactive_clients(&self, timeout: Duration) -> Vec<ClientId> {
        let state = self.state.lock().await;
        let now = Instant::now();

        state.last_client_activity
            .iter()
            .filter(|(_, last_activity)| now.duration_since(**last_activity) > timeout)
            .map(|(client_id, _)| client_id.clone())
            .collect()
    }

    pub async fn get_inactive_lobbies(&self, timeout: Duration) -> Vec<LobbyId> {
        let state = self.state.lock().await;
        let now = Instant::now();

        state.last_lobby_activity
            .iter()
            .filter(|(_, last_activity)| now.duration_since(**last_activity) > timeout)
            .map(|(lobby_id, _)| lobby_id.clone())
            .collect()
    }

    pub async fn get_lobby_players(&self, lobby_id: &LobbyId) -> Vec<ClientId> {
        let state = self.state.lock().await;

        state.client_to_lobby
            .iter()
            .filter(|(_, lid)| *lid == lobby_id)
            .map(|(cid, _)| cid.clone())
            .collect()
    }

    pub async fn create_lobby(&self, name: String, max_players: u32, settings: LobbySettings, creator_id: ClientId) -> Result<LobbyDetails, String> {
        settings.validate(max_players)?;

        let mut state = self.state.lock().await;

        if state.client_to_lobby.contains_key(&creator_id) {
            return Err("Already in a lobby".to_string());
        }

        let lobby_id = LobbyId::new(format!("lobby_{}", state.next_lobby_id));
        state.next_lobby_id += 1;

        let mut lobby = Lobby::new(lobby_id.clone(), name, creator_id.clone(), max_players, settings);
        let creator_player_id = PlayerId::new(creator_id.to_string());
        lobby.add_player(creator_player_id.clone());
        lobby.set_ready(&creator_player_id, true);

        let details = lobby.to_details();

        state.lobbies.insert(lobby_id.clone(), lobby);
        state.client_to_lobby.insert(creator_id.clone(), lobby_id.clone());
        state.clients_not_in_lobby.remove(&creator_id);
        state.last_lobby_activity.insert(lobby_id, Instant::now());

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

    pub async fn join_lobby(&self, lobby_id: LobbyId, client_id: ClientId, join_as_observer: bool) -> Result<LobbyDetails, String> {
        let mut state = self.state.lock().await;

        if state.client_to_lobby.contains_key(&client_id) {
            return Err("Already in a lobby".to_string());
        }

        let lobby = state.lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

        if lobby.has_ever_started() {
            return Err("Cannot join: Lobby no longer accepting new players".to_string());
        }

        let player_id = PlayerId::new(client_id.to_string());

        if join_as_observer {
            if !lobby.add_observer(player_id) {
                return Err("Already in lobby".to_string());
            }
        } else if !lobby.add_player(player_id) {
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
            let is_host = lobby.is_host(client_id);
            let player_id = PlayerId::new(client_id.to_string());

            let was_observer = lobby.remove_observer(&player_id);
            if !was_observer {
                lobby.remove_player(&player_id);
            }

            if is_host {
                let kicked_players: Vec<ClientId> = lobby.players.keys()
                    .map(|player_id| ClientId::new(player_id.to_string()))
                    .collect();
                let kicked_observers: Vec<ClientId> = lobby.observers.iter()
                    .map(|player_id| ClientId::new(player_id.to_string()))
                    .collect();
                LobbyStateAfterLeave::HostLeft {
                    kicked_players: kicked_players.into_iter().chain(kicked_observers).collect(),
                }
            } else if lobby.players.is_empty() && lobby.bots.is_empty() {
                let kicked_observers: Vec<ClientId> = lobby.observers.iter()
                    .map(|player_id| ClientId::new(player_id.to_string()))
                    .collect();
                LobbyStateAfterLeave::HostLeft {
                    kicked_players: kicked_observers,
                }
            } else {
                LobbyStateAfterLeave::LobbyStillActive {
                    updated_details: lobby.to_details(),
                }
            }
        };

        state.clients_not_in_lobby.insert(client_id.clone());

        match &result {
            LobbyStateAfterLeave::HostLeft { kicked_players } => {
                for kicked_id in kicked_players {
                    state.client_to_lobby.remove(kicked_id);
                    state.clients_not_in_lobby.insert(kicked_id.clone());
                }
                state.lobbies.remove(&lobby_id);
                state.last_lobby_activity.remove(&lobby_id);
            }
            LobbyStateAfterLeave::LobbyStillActive { .. } => {}
        }

        Ok(result)
    }

    pub async fn mark_ready(&self, client_id: &ClientId, ready: bool) -> Result<LobbyDetails, String> {
        let mut state = self.state.lock().await;

        let lobby_id = state.client_to_lobby.get(client_id).cloned().ok_or("Not in a lobby")?;

        let lobby = state.lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

        let player_id = PlayerId::new(client_id.to_string());
        if !lobby.set_ready(&player_id, ready) {
            return Err("Player not in lobby".to_string());
        }

        Ok(lobby.to_details())
    }

    pub async fn add_bot(&self, client_id: &ClientId, bot_type: BotType) -> Result<(LobbyDetails, PlayerIdentity), String> {
        let mut state = self.state.lock().await;

        let lobby_id = state.client_to_lobby.get(client_id).cloned().ok_or("Not in a lobby")?;

        let bot_id_number = state.next_bot_id;
        state.next_bot_id += 1;

        let lobby = state.lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

        if !lobby.is_host(client_id) {
            return Err("Only the host can add bots".to_string());
        }

        let bot_id = BotId::new(format!("{} Bot-{}", generate_client_id(), bot_id_number));

        if !lobby.add_bot_with_id(bot_id.clone(), bot_type) {
            return Err("Cannot add bot: lobby full or bot already exists".to_string());
        }

        let bot_identity = PlayerIdentity::Bot {
            id: bot_id,
            bot_type,
        };

        Ok((lobby.to_details(), bot_identity))
    }

    pub async fn kick_from_lobby(&self, client_id: &ClientId, target_id: String) -> Result<(LobbyDetails, PlayerIdentity, bool), String> {
        let mut state = self.state.lock().await;

        let lobby_id = state.client_to_lobby.get(client_id).cloned().ok_or("Not in a lobby")?;
        let lobby = state.lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

        if !lobby.is_host(client_id) {
            return Err("Only the host can kick players".to_string());
        }

        let player_id = PlayerId::new(target_id.clone());
        let bot_id = BotId::new(target_id.clone());

        let (identity, is_bot, is_observer) = if lobby.players.contains_key(&player_id) {
            (PlayerIdentity::Player(player_id.clone()), false, false)
        } else if let Some(bot_type) = lobby.bots.get(&bot_id) {
            (PlayerIdentity::Bot { id: bot_id.clone(), bot_type: *bot_type }, true, false)
        } else if lobby.observers.contains(&player_id) {
            (PlayerIdentity::Player(player_id.clone()), false, true)
        } else {
            return Err("Player not in lobby".to_string());
        };

        if is_bot {
            lobby.remove_bot(&bot_id);
            let lobby_details = lobby.to_details();
            Ok((lobby_details, identity, is_bot))
        } else if is_observer {
            lobby.remove_observer(&player_id);
            let lobby_details = lobby.to_details();
            let target_client_id = ClientId::new(target_id);
            state.client_to_lobby.remove(&target_client_id);
            state.clients_not_in_lobby.insert(target_client_id);
            Ok((lobby_details, identity, is_bot))
        } else {
            lobby.remove_player(&player_id);
            let lobby_details = lobby.to_details();
            let target_client_id = ClientId::new(target_id);
            state.client_to_lobby.remove(&target_client_id);
            state.clients_not_in_lobby.insert(target_client_id);
            Ok((lobby_details, identity, is_bot))
        }
    }

    pub async fn start_game(&self, client_id: &ClientId) -> Result<LobbyId, String> {
        let mut state = self.state.lock().await;

        let lobby_id = state.client_to_lobby.get(client_id).cloned().ok_or("Not in a lobby")?;

        let lobby = state.lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

        if !lobby.is_host(client_id) {
            return Err("Only the host can start the game".to_string());
        }

        if lobby.in_game {
            return Err("Game already started".to_string());
        }

        if !lobby.all_players_ready() {
            return Err("Not all players are ready".to_string());
        }

        let total_players = lobby.total_player_count();
        match &lobby.settings {
            LobbySettings::TicTacToe(_) => {
                if total_players != 2 {
                    return Err(format!("TicTacToe requires exactly 2 players, but {} are in the lobby", total_players));
                }
            }
            LobbySettings::Snake(_) => {
                if total_players == 0 {
                    return Err("Cannot start game with no players".to_string());
                }
            }
            LobbySettings::NumbersMatch(_) => {
                if total_players != 1 {
                    return Err(format!("NumbersMatch requires exactly 1 player, but {} are in the lobby", total_players));
                }
            }
            LobbySettings::StackAttack(_) => {
                use common::games::stack_attack::settings::{MIN_PLAYERS, MAX_PLAYERS};
                if total_players < MIN_PLAYERS {
                    return Err(format!("Stack Attack requires at least {} player(s)", MIN_PLAYERS));
                }
                if total_players > MAX_PLAYERS {
                    return Err(format!("Stack Attack allows at most {} players, but {} are in the lobby", MAX_PLAYERS, total_players));
                }
            }
            LobbySettings::Puzzle2048(_) => {
                if total_players != 1 {
                    return Err(format!("Puzzle 2048 requires exactly 1 player, but {} are in the lobby", total_players));
                }
            }
        }

        lobby.start_game();

        Ok(lobby_id)
    }

    pub async fn end_game(&self, lobby_id: &LobbyId) -> Result<Vec<PlayerId>, String> {
        let mut state = self.state.lock().await;

        let lobby = state.lobbies.get_mut(lobby_id).ok_or("Lobby not found")?;
        let player_ids: Vec<PlayerId> = lobby.players.keys().cloned().collect();

        lobby.end_game();

        Ok(player_ids)
    }

    pub async fn vote_play_again(&self, client_id: &ClientId) -> Result<(LobbyId, PlayAgainStatus), String> {
        let mut state = self.state.lock().await;
        let lobby_id = state.client_to_lobby.get(client_id).ok_or("Not in a lobby")?.clone();

        let lobby = state.lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

        if lobby.in_game {
            return Err("Game is still in progress".to_string());
        }

        let player_id = PlayerId::new(client_id.to_string());

        if !lobby.vote_play_again(&player_id) {
            if !lobby.original_game_players.contains(&player_id) {
                return Err("Player was not in the original game".to_string());
            }
            if !lobby.players.contains_key(&player_id) {
                return Err("Player is no longer in the lobby".to_string());
            }
        }

        if !lobby.is_play_again_available() {
            return Ok((lobby_id, PlayAgainStatus::NotAvailable));
        }

        Ok((lobby_id, lobby.get_play_again_status()))
    }

    pub async fn get_play_again_status(&self, lobby_id: &LobbyId) -> Result<PlayAgainStatus, String> {
        let state = self.state.lock().await;
        let lobby = state.lobbies.get(lobby_id).ok_or("Lobby not found")?;
        Ok(lobby.get_play_again_status())
    }

    pub async fn become_observer(&self, client_id: &ClientId) -> Result<LobbyDetails, String> {
        let mut state = self.state.lock().await;
        let lobby_id = state.client_to_lobby.get(client_id).cloned().ok_or("Not in a lobby")?;
        let lobby = state.lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

        let player_id = PlayerId::new(client_id.to_string());
        if !lobby.player_to_observer(&player_id) {
            return Err("Not a player in this lobby".to_string());
        }

        Ok(lobby.to_details())
    }

    pub async fn become_player(&self, client_id: &ClientId) -> Result<LobbyDetails, String> {
        let mut state = self.state.lock().await;
        let lobby_id = state.client_to_lobby.get(client_id).cloned().ok_or("Not in a lobby")?;
        let lobby = state.lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

        let player_id = PlayerId::new(client_id.to_string());
        if !lobby.observer_to_player(&player_id) {
            return Err("Cannot become player: not an observer or lobby is full".to_string());
        }

        Ok(lobby.to_details())
    }

    pub async fn make_player_observer(&self, client_id: &ClientId, target_id: String) -> Result<LobbyDetails, String> {
        let mut state = self.state.lock().await;
        let lobby_id = state.client_to_lobby.get(client_id).cloned().ok_or("Not in a lobby")?;
        let lobby = state.lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

        if !lobby.is_host(client_id) {
            return Err("Only the host can make players observers".to_string());
        }

        if lobby.creator_id.to_string() == target_id {
            return Err("Cannot make host an observer".to_string());
        }

        let target_player_id = PlayerId::new(target_id);
        if !lobby.player_to_observer(&target_player_id) {
            return Err("Target is not a player in this lobby".to_string());
        }

        Ok(lobby.to_details())
    }

    pub async fn get_client_lobby(&self, client_id: &ClientId) -> Option<LobbyDetails> {
        let state = self.state.lock().await;
        let lobby_id = state.client_to_lobby.get(client_id);

        if let Some(lobby_id) = lobby_id {
            state.lobbies.get(lobby_id).map(|lobby| lobby.to_details())
        } else {
            None
        }
    }

    pub async fn get_lobby(&self, lobby_id: &LobbyId) -> Option<Lobby> {
        let state = self.state.lock().await;
        state.lobbies.get(lobby_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{WallCollisionMode, DeadSnakeBehavior, SnakeLobbySettings};

    fn default_test_settings() -> LobbySettings {
        LobbySettings::Snake(SnakeLobbySettings {
            field_width: 15,
            field_height: 15,
            wall_collision_mode: WallCollisionMode::WrapAround.into(),
            tick_interval_ms: 200,
            max_food_count: 5,
            food_spawn_probability: 0.5,
            dead_snake_behavior: DeadSnakeBehavior::Disappear.into(),
        })
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
        assert_eq!(details.creator.as_ref().unwrap().player_id, creator_id.to_string());
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
}
