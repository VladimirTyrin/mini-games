use crate::{
    PlayerIdentity, ReplayV1, ReplayV1Metadata, PlayerAction, PlayerActionContent,
    player_action_content, ReplayGame, InGameCommand, PlayerDisconnected,
    lobby_settings,
};
use std::collections::HashMap;

pub struct ReplayRecorder {
    engine_version: String,
    game_started_timestamp_ms: i64,
    game: ReplayGame,
    seed: u64,
    lobby_settings: Option<lobby_settings::Settings>,
    players: Vec<PlayerIdentity>,
    actions: Vec<PlayerAction>,
    player_index_map: HashMap<String, i32>,
}

impl ReplayRecorder {
    pub fn new(
        engine_version: String,
        game: ReplayGame,
        seed: u64,
        lobby_settings: Option<lobby_settings::Settings>,
        players: Vec<PlayerIdentity>,
    ) -> Self {
        let game_started_timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let index_map = players
            .iter()
            .enumerate()
            .map(|(i, p)| (p.player_id.clone(), i as i32))
            .collect();
        
        Self {
            engine_version,
            game_started_timestamp_ms,
            game,
            seed,
            lobby_settings,
            players,
            actions: Vec::new(),
            player_index_map: index_map,
        }
    }

    pub fn record_command(&mut self, tick: i64, player_index: i32, command: InGameCommand) {
        self.actions.push(PlayerAction {
            tick,
            player_index,
            content: Some(PlayerActionContent {
                content: Some(player_action_content::Content::Command(command)),
            }),
        });
    }

    pub fn record_disconnect(&mut self, tick: i64, player_index: i32) {
        self.actions.push(PlayerAction {
            tick,
            player_index,
            content: Some(PlayerActionContent {
                content: Some(player_action_content::Content::Disconnected(PlayerDisconnected {})),
            }),
        });
    }

    pub fn find_player_index(&self, player_id: &str) -> Option<i32> {
        self.player_index_map.get(player_id).copied()
    }

    pub fn finalize(&mut self) -> ReplayV1 {
        let mut actions = std::mem::take(&mut self.actions);
        actions.sort_by_key(|a| a.tick);

        ReplayV1 {
            metadata: Some(ReplayV1Metadata {
                engine_version: std::mem::take(&mut self.engine_version),
                game_started_timestamp_ms: self.game_started_timestamp_ms,
                game: self.game.into(),
                seed: self.seed,
                lobby_settings: self.lobby_settings.take().map(|s| crate::LobbySettings { settings: Some(s) }),
                players: std::mem::take(&mut self.players),
            }),
            actions,
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn actions_count(&self) -> usize {
        self.actions.len()
    }
}
