use std::collections::{HashMap, HashSet};
use crate::{
    LobbyInfo, LobbyDetails, PlayerInfo, ClientId, LobbyId, PlayerId, BotId,
    SnakeLobbySettings, SnakeBotType, TicTacToeLobbySettings, TicTacToeBotType,
    lobby_details, lobby_settings, add_bot_request,
    validate_lobby_settings::ValidateLobbySettings,
};
use crate::id_generator::generate_client_id;

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

    pub fn to_info_proto(&self) -> Option<crate::proto::game_service::LobbySettings> {
        Some(crate::proto::game_service::LobbySettings {
            settings: Some(match self {
                LobbySettings::Snake(s) => lobby_settings::Settings::Snake(*s),
                LobbySettings::TicTacToe(t) => lobby_settings::Settings::Tictactoe(*t),
            }),
        })
    }

    pub fn from_proto(settings: Option<lobby_settings::Settings>) -> Result<Self, String> {
        match settings {
            Some(lobby_settings::Settings::Snake(s)) => Ok(LobbySettings::Snake(s)),
            Some(lobby_settings::Settings::Tictactoe(t)) => Ok(LobbySettings::TicTacToe(t)),
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

    pub fn to_proto(&self) -> crate::PlayerIdentity {
        crate::PlayerIdentity {
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
    pub observers: HashSet<PlayerId>,
    pub in_game: bool,
    pub play_again_votes: HashSet<PlayerId>,
    pub original_game_players: HashSet<PlayerId>,
}

#[derive(Debug)]
pub enum LobbyStateAfterLeave {
    LobbyStillActive { updated_details: LobbyDetails },
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
    pub fn new(id: LobbyId, name: String, creator_id: ClientId, max_players: u32, settings: LobbySettings) -> Self {
        Self {
            id,
            name,
            creator_id,
            max_players,
            settings,
            players: HashMap::new(),
            bots: HashMap::new(),
            observers: HashSet::new(),
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
            observer_count: self.observers.len() as u32,
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

        let observers: Vec<crate::PlayerIdentity> = self.observers.iter()
            .map(|id| crate::PlayerIdentity {
                player_id: id.to_string(),
                is_bot: false,
            })
            .collect();

        let creator_identity = crate::PlayerIdentity {
            player_id: self.creator_id.to_string(),
            is_bot: false,
        };

        LobbyDetails {
            lobby_id: self.id.to_string(),
            lobby_name: self.name.clone(),
            players: all_players,
            max_players: self.max_players,
            observers,
            settings: self.settings.to_proto(),
            creator: Some(creator_identity),
        }
    }

    pub fn add_player(&mut self, player_id: PlayerId) -> bool {
        if (self.players.len() + self.bots.len()) >= self.max_players as usize {
            return false;
        }
        if self.players.contains_key(&player_id) {
            return false;
        }
        self.players.insert(player_id, false);
        true
    }

    pub fn remove_player(&mut self, player_id: &PlayerId) -> bool {
        self.players.remove(player_id).is_some()
    }

    pub fn remove_bot(&mut self, bot_id: &BotId) -> bool {
        self.bots.remove(bot_id).is_some()
    }

    pub fn set_ready(&mut self, player_id: &PlayerId, ready: bool) -> bool {
        if let Some(player_ready) = self.players.get_mut(player_id) {
            *player_ready = ready;
            true
        } else {
            false
        }
    }

    pub fn add_bot(&mut self, bot_type: BotType) -> Option<BotId> {
        if (self.players.len() + self.bots.len()) >= self.max_players as usize {
            return None;
        }
        let bot_id = BotId::new(format!("{} Bot-{}", generate_client_id(), self.bots.len() + 1));
        self.bots.insert(bot_id.clone(), bot_type);
        Some(bot_id)
    }

    pub fn add_bot_with_id(&mut self, bot_id: BotId, bot_type: BotType) -> bool {
        if (self.players.len() + self.bots.len()) >= self.max_players as usize {
            return false;
        }
        if self.bots.contains_key(&bot_id) {
            return false;
        }
        self.bots.insert(bot_id, bot_type);
        true
    }

    pub fn has_ever_started(&self) -> bool {
        !self.original_game_players.is_empty()
    }

    pub fn add_observer(&mut self, player_id: PlayerId) -> bool {
        if self.players.contains_key(&player_id) || self.observers.contains(&player_id) {
            return false;
        }
        self.observers.insert(player_id);
        true
    }

    pub fn remove_observer(&mut self, player_id: &PlayerId) -> bool {
        self.observers.remove(player_id)
    }

    pub fn player_to_observer(&mut self, player_id: &PlayerId) -> bool {
        if self.players.remove(player_id).is_some() {
            self.observers.insert(player_id.clone());
            true
        } else {
            false
        }
    }

    pub fn observer_to_player(&mut self, player_id: &PlayerId) -> bool {
        if !self.observers.contains(player_id) {
            return false;
        }
        let current_player_count = self.players.len() + self.bots.len();
        if current_player_count >= self.max_players as usize {
            return false;
        }
        self.observers.remove(player_id);
        self.players.insert(player_id.clone(), false);
        true
    }

    pub fn get_pending_for_play_again(&self) -> Vec<String> {
        self.original_game_players.iter()
            .filter(|id| !self.play_again_votes.contains(id))
            .map(|id| id.to_string())
            .collect()
    }

    pub fn all_players_ready(&self) -> bool {
        self.players.values().all(|ready| *ready)
    }

    pub fn total_player_count(&self) -> usize {
        self.players.len() + self.bots.len()
    }

    pub fn is_host(&self, client_id: &ClientId) -> bool {
        &self.creator_id == client_id
    }

    pub fn start_game(&mut self) {
        self.in_game = true;
        self.play_again_votes.clear();
        self.original_game_players = self.players.keys().cloned().collect();
    }

    pub fn end_game(&mut self) {
        self.in_game = false;
        for ready in self.players.values_mut() {
            *ready = false;
        }
    }

    pub fn vote_play_again(&mut self, player_id: &PlayerId) -> bool {
        if !self.original_game_players.contains(player_id) {
            return false;
        }
        if !self.players.contains_key(player_id) {
            return false;
        }
        self.play_again_votes.insert(player_id.clone());
        self.set_ready(player_id, true);
        true
    }

    pub fn is_play_again_available(&self) -> bool {
        !self.original_game_players.is_empty()
            && self.players.len() == self.original_game_players.len()
    }

    pub fn get_play_again_status(&self) -> PlayAgainStatus {
        if !self.is_play_again_available() {
            return PlayAgainStatus::NotAvailable;
        }

        let ready_player_ids: Vec<String> = self.play_again_votes.iter()
            .map(|id| id.to_string())
            .collect();
        let pending_player_ids: Vec<String> = self.get_pending_for_play_again();

        PlayAgainStatus::Available {
            ready_player_ids,
            pending_player_ids,
        }
    }
}
