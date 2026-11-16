use crate::game::Game;
use crate::player::PlayerSpec;

pub struct Setup<G: Game> {
    pub settings: G::Settings,
    pub players: Vec<PlayerSpec>,
}

impl<G: Game> Setup<G> {
    pub fn new(settings: G::Settings, players: Vec<PlayerSpec>) -> Self {
        Self { settings, players }
    }
}

impl<G: Game> std::fmt::Debug for Setup<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Setup")
            .field("settings", &self.settings)
            .field("players", &self.players)
            .finish()
    }
}
