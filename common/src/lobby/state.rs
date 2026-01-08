use std::collections::{HashMap, HashSet};
use crate::{LobbyInfo, LobbyDetails, PlayerInfo, ClientId, LobbyId, PlayerId, BotId};
use crate::id_generator::generate_client_id;
use super::{LobbySettings, BotType, PlayerIdentity};

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

        let observers: Vec<crate::proto::game_service::PlayerIdentity> = self.observers.iter()
            .map(|id| crate::proto::game_service::PlayerIdentity {
                player_id: id.to_string(),
                is_bot: false,
            })
            .collect();

        let creator_identity = crate::proto::game_service::PlayerIdentity {
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
