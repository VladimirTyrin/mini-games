pub mod snake_session;
pub mod tictactoe_session;

use std::collections::{HashMap, HashSet};
use std::future::Future;
use crate::{ClientId, PlayerId, BotId, GameStateUpdate, GameOverNotification};
use crate::lobby::BotType;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

pub struct SessionRng {
    rng: StdRng,
    seed: u64,
}

impl SessionRng {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
            seed,
        }
    }

    pub fn from_random() -> Self {
        let seed: u64 = rand::rng().random();
        Self::new(seed)
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn random<T>(&mut self) -> T
    where
        rand::distr::StandardUniform: rand::distr::Distribution<T>,
    {
        self.rng.random()
    }

    pub fn random_range<T, R>(&mut self, range: R) -> T
    where
        T: rand::distr::uniform::SampleUniform,
        R: rand::distr::uniform::SampleRange<T>,
    {
        self.rng.random_range(range)
    }

    pub fn random_bool(&mut self) -> bool {
        self.rng.random()
    }
}

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
