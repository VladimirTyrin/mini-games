use std::collections::{HashMap, HashSet};

use crate::{BotId, ClientId, PlayerId};
use crate::games::BotType;

#[derive(Debug, Clone)]
pub struct GameSessionConfig {
    pub session_id: String,
    pub human_players: Vec<PlayerId>,
    pub observers: HashSet<PlayerId>,
    pub bots: HashMap<BotId, BotType>,
}

impl GameSessionConfig {
    pub fn get_all_recipients(&self) -> Vec<ClientId> {
        let mut recipients: Vec<ClientId> = self
            .human_players
            .iter()
            .map(|p| ClientId::new(p.to_string()))
            .collect();
        recipients.extend(
            self.observers
                .iter()
                .map(|p| ClientId::new(p.to_string())),
        );
        recipients
    }
}
