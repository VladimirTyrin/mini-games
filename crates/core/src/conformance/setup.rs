use crate::game::Game;
use crate::game_settings::GameSettings;
use crate::player_spec::PlayerSpec;

/// Settings in their raw form: the suite parses them like any boundary would.
pub struct Setup<G: Game> {
    pub settings: <G::Settings as GameSettings>::Raw,
    pub players: Vec<PlayerSpec>,
}

impl<G: Game> Setup<G> {
    pub fn new(settings: <G::Settings as GameSettings>::Raw, players: Vec<PlayerSpec>) -> Self {
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
