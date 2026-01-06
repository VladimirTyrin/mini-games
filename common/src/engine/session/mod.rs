pub mod snake_session;
pub mod tictactoe_session;

use std::collections::{HashMap, HashSet};
use std::future::Future;
use crate::{ClientId, PlayerId, BotId, GameStateUpdate, GameOverNotification};
use crate::lobby::BotType;

pub trait GameBroadcaster: Send + Sync + Clone + 'static {
    fn broadcast_state(
        &self,
        state: GameStateUpdate,
        recipients: Vec<ClientId>,
    ) -> impl Future<Output = ()> + Send;

    fn broadcast_game_over(
        &self,
        notification: GameOverNotification,
        recipients: Vec<ClientId>,
    ) -> impl Future<Output = ()> + Send;
}

#[derive(Debug, Clone)]
pub struct GameSessionConfig {
    pub session_id: String,
    pub human_players: Vec<PlayerId>,
    pub observers: HashSet<PlayerId>,
    pub bots: HashMap<BotId, BotType>,
}

impl GameSessionConfig {
    pub fn get_all_recipients(&self) -> Vec<ClientId> {
        let mut recipients: Vec<ClientId> = self.human_players.iter()
            .map(|p| ClientId::new(p.to_string()))
            .collect();
        recipients.extend(self.observers.iter().map(|p| ClientId::new(p.to_string())));
        recipients
    }
}
