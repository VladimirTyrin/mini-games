use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use common::{
    LobbyInfo, LobbyDetails, PlayerInfo, ClientId, LobbyId, PlayerId, BotId,
    SnakeLobbySettings, SnakeBotType, TicTacToeLobbySettings, TicTacToeBotType,
    lobby_info, lobby_details, create_lobby_request, add_bot_request,
    validation::ValidateLobbySettings,
};
use common::id_generator::generate_client_id;

#[derive(Debug, Clone)]
pub enum LobbySettings {
    Snake(SnakeLobbySettings),
    TicTacToe(TicTacToeLobbySettings),
}

impl LobbySettings {
    pub fn validate(&self, max_players: u32) -> Result<(), String> {
        match self {
            LobbySettings::Snake(s) => s.validate(max_players),
            LobbySettings::TicTacToe(t) => t.validate(max_players),
        }
    }

    pub fn to_proto(&self) -> Option<lobby_details::Settings> {
        match self {
            LobbySettings::Snake(s) => Some(lobby_details::Settings::Snake(*s)),
            LobbySettings::TicTacToe(t) => Some(lobby_details::Settings::Tictactoe(*t)),
        }
    }

    pub fn to_info_proto(&self) -> Option<lobby_info::Settings> {
        match self {
            LobbySettings::Snake(s) => Some(lobby_info::Settings::Snake(*s)),
            LobbySettings::TicTacToe(t) => Some(lobby_info::Settings::Tictactoe(*t)),
        }
    }

    pub fn from_proto(settings: Option<create_lobby_request::Settings>) -> Result<Self, String> {
        match settings {
            Some(create_lobby_request::Settings::Snake(s)) => Ok(LobbySettings::Snake(s)),
            Some(create_lobby_request::Settings::Tictactoe(t)) => Ok(LobbySettings::TicTacToe(t)),
            None => Err("No settings provided".to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotType {
    Snake(SnakeBotType),
    TicTacToe(TicTacToeBotType),
}

impl BotType {
    pub fn from_proto(bot_type: Option<add_bot_request::BotType>) -> Result<Self, String> {
        match bot_type {
            Some(add_bot_request::BotType::SnakeBot(t)) => Ok(BotType::Snake(
                SnakeBotType::try_from(t).map_err(|_| "Invalid snake bot type")?
            )),
            Some(add_bot_request::BotType::TictactoeBot(t)) => Ok(BotType::TicTacToe(
                TicTacToeBotType::try_from(t).map_err(|_| "Invalid tictactoe bot type")?
            )),
            None => Err("No bot type provided".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerIdentity {
    Player(PlayerId),
    Bot { id: BotId, bot_type: BotType },
}

impl PlayerIdentity {
    pub fn client_id(&self) -> String {
        match self {
            PlayerIdentity::Player(id) => id.to_string(),
            PlayerIdentity::Bot { id, .. } => id.to_string(),
        }
    }

    pub fn to_proto(&self) -> common::PlayerIdentity {
        common::PlayerIdentity {
            player_id: self.client_id(),
            is_bot: matches!(self, PlayerIdentity::Bot { .. }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Lobby {
    pub id: LobbyId,
    pub name: String,
    pub creator_id: ClientId,
    pub max_players: u32,
    pub settings: LobbySettings,
    pub players: HashMap<PlayerId, bool>,
    pub bots: HashMap<BotId, BotType>,
    pub in_game: bool,
    pub play_again_votes: HashSet<PlayerId>,
    pub original_game_players: HashSet<PlayerId>,
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
            bots: HashMap::new(),
            in_game: false,
            play_again_votes: HashSet::new(),
            original_game_players: HashSet::new(),
        }
    }

    pub fn to_info(&self) -> LobbyInfo {
        LobbyInfo {
            lobby_id: self.id.to_string(),
            lobby_name: self.name.clone(),
            current_players: (self.players.len() + self.bots.len()) as u32,
            max_players: self.max_players,
            settings: self.settings.to_info_proto(),
        }
    }

    pub fn to_details(&self) -> LobbyDetails {
        let mut all_players: Vec<PlayerInfo> = Vec::new();

        for (player_id, ready) in &self.players {
            all_players.push(PlayerInfo {
                identity: Some(PlayerIdentity::Player(player_id.clone()).to_proto()),
                ready: *ready,
            });
        }

        for (bot_id, bot_type) in &self.bots {
            all_players.push(PlayerInfo {
                identity: Some(PlayerIdentity::Bot {
                    id: bot_id.clone(),
                    bot_type: *bot_type
                }.to_proto()),
                ready: true,
            });
        }

        let creator_identity = common::PlayerIdentity {
            player_id: self.creator_id.to_string(),
            is_bot: false,
        };

        LobbyDetails {
            lobby_id: self.id.to_string(),
            lobby_name: self.name.clone(),
            players: all_players,
            max_players: self.max_players,
            settings: self.settings.to_proto(),
            creator: Some(creator_identity),
        }
    }

    fn add_player(&mut self, player_id: PlayerId) -> bool {
        if (self.players.len() + self.bots.len()) >= self.max_players as usize {
            return false;
        }
        if self.players.contains_key(&player_id) {
            return false;
        }
        self.players.insert(player_id, false);
        true
    }

    fn remove_player(&mut self, player_id: &PlayerId) -> bool {
        self.players.remove(player_id).is_some()
    }

    fn remove_bot(&mut self, bot_id: &BotId) -> bool {
        self.bots.remove(bot_id).is_some()
    }

    fn set_ready(&mut self, player_id: &PlayerId, ready: bool) -> bool {
        if let Some(player_ready) = self.players.get_mut(player_id) {
            *player_ready = ready;
            true
        } else {
            false
        }
    }

    fn add_bot(&mut self, bot_id: BotId, bot_type: BotType) -> bool {
        if (self.players.len() + self.bots.len()) >= self.max_players as usize {
            return false;
        }
        if self.bots.contains_key(&bot_id) {
            return false;
        }
        self.bots.insert(bot_id, bot_type);
        true
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
    next_bot_id: u64,
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
                next_bot_id: 1,
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

        let player_id = PlayerId::new(client_id.to_string());
        if !lobby.add_player(player_id) {
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
            let player_id = PlayerId::new(client_id.to_string());
            lobby.remove_player(&player_id);

            if is_host {
                let kicked_players: Vec<ClientId> = lobby.players.keys()
                    .map(|player_id| ClientId::new(player_id.to_string()))
                    .collect();
                LobbyStateAfterLeave::HostLeft {
                    kicked_players,
                }
            } else if lobby.players.is_empty() && lobby.bots.is_empty() {
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
                for client_id in kicked_players {
                    state.client_to_lobby.remove(client_id);
                    state.clients_not_in_lobby.insert(client_id.clone());
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

        if &lobby.creator_id != client_id {
            return Err("Only the host can add bots".to_string());
        }

        let bot_id = BotId::new(format!("{} Bot-{}", generate_client_id(), bot_id_number));

        if !lobby.add_bot(bot_id.clone(), bot_type) {
            return Err("Cannot add bot: lobby full or bot already exists".to_string());
        }

        let bot_identity = PlayerIdentity::Bot {
            id: bot_id.clone(),
            bot_type,
        };

        Ok((lobby.to_details(), bot_identity))
    }

    pub async fn kick_from_lobby(&self, client_id: &ClientId, target_id: String) -> Result<(LobbyDetails, PlayerIdentity, bool), String> {
        let mut state = self.state.lock().await;

        let lobby_id = state.client_to_lobby.get(client_id).cloned().ok_or("Not in a lobby")?;
        let lobby = state.lobbies.get_mut(&lobby_id).ok_or("Lobby not found")?;

        if &lobby.creator_id != client_id {
            return Err("Only the host can kick players".to_string());
        }

        let player_id = PlayerId::new(target_id.clone());
        let bot_id = BotId::new(target_id.clone());

        let (identity, is_bot) = if lobby.players.contains_key(&player_id) {
            (PlayerIdentity::Player(player_id.clone()), false)
        } else if let Some(bot_type) = lobby.bots.get(&bot_id) {
            (PlayerIdentity::Bot { id: bot_id.clone(), bot_type: *bot_type }, true)
        } else {
            return Err("Player not in lobby".to_string());
        };

        if is_bot {
            lobby.remove_bot(&bot_id);
            let lobby_details = lobby.to_details();
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

        let total_players = lobby.players.len() + lobby.bots.len();
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
        }

        lobby.in_game = true;
        lobby.play_again_votes.clear();

        lobby.original_game_players = lobby.players.keys().cloned().collect();

        Ok(lobby_id)
    }

    pub async fn end_game(&self, lobby_id: &LobbyId) -> Result<Vec<PlayerId>, String> {
        let mut state = self.state.lock().await;

        let lobby = state.lobbies.get_mut(lobby_id).ok_or("Lobby not found")?;
        let player_ids: Vec<PlayerId> = lobby.players.keys().cloned().collect();

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

        let player_id = PlayerId::new(client_id.to_string());
        if !lobby.original_game_players.contains(&player_id) {
            return Err("Player was not in the original game".to_string());
        }

        if !lobby.players.contains_key(&player_id) {
            return Err("Player is no longer in the lobby".to_string());
        }

        let play_again_available = lobby.players.len() == lobby.original_game_players.len();
        if !play_again_available {
            return Ok((lobby_id, PlayAgainStatus::NotAvailable));
        }

        lobby.play_again_votes.insert(player_id.clone());
        lobby.set_ready(&player_id, true);

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
    use common::WallCollisionMode;
    use common::DeadSnakeBehavior;

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
