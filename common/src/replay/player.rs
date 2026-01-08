use crate::{ReplayV1, ReplayV1Metadata, PlayerAction, PlayerIdentity, lobby_settings, ReplayGame};

pub struct ReplayPlayer {
    replay: ReplayV1,
    current_action_index: usize,
}

impl ReplayPlayer {
    pub fn new(replay: ReplayV1) -> Self {
        Self {
            replay,
            current_action_index: 0,
        }
    }

    fn metadata(&self) -> &ReplayV1Metadata {
        self.replay.metadata.as_ref().expect("Replay must have metadata")
    }

    pub fn engine_version(&self) -> &str {
        &self.metadata().engine_version
    }

    pub fn game(&self) -> ReplayGame {
        ReplayGame::try_from(self.metadata().game).unwrap_or(ReplayGame::Unspecified)
    }

    pub fn seed(&self) -> u64 {
        self.metadata().seed
    }

    pub fn lobby_settings(&self) -> Option<&lobby_settings::Settings> {
        self.metadata().lobby_settings.as_ref().and_then(|s| s.settings.as_ref())
    }

    pub fn players(&self) -> &[PlayerIdentity] {
        &self.metadata().players
    }

    pub fn get_player(&self, index: i32) -> Option<&PlayerIdentity> {
        self.metadata().players.get(index as usize)
    }

    pub fn game_started_timestamp_ms(&self) -> i64 {
        self.metadata().game_started_timestamp_ms
    }

    pub fn total_actions(&self) -> usize {
        self.replay.actions.len()
    }

    pub fn current_action_index(&self) -> usize {
        self.current_action_index
    }

    pub fn is_finished(&self) -> bool {
        self.current_action_index >= self.replay.actions.len()
    }

    pub fn peek_next_action(&self) -> Option<&PlayerAction> {
        self.replay.actions.get(self.current_action_index)
    }

    pub fn next_action(&mut self) -> Option<&PlayerAction> {
        if self.current_action_index < self.replay.actions.len() {
            let action = &self.replay.actions[self.current_action_index];
            self.current_action_index += 1;
            Some(action)
        } else {
            None
        }
    }

    pub fn actions_for_tick(&mut self, tick: i64) -> Vec<PlayerAction> {
        let mut actions = Vec::new();
        while self.current_action_index < self.replay.actions.len() {
            let action = &self.replay.actions[self.current_action_index];
            if action.tick == tick {
                actions.push(*action);
                self.current_action_index += 1;
            } else if action.tick > tick {
                break;
            } else {
                self.current_action_index += 1;
            }
        }
        actions
    }

    pub fn reset(&mut self) {
        self.current_action_index = 0;
    }

    pub fn into_replay(self) -> ReplayV1 {
        self.replay
    }

    pub fn replay_ref(&self) -> &ReplayV1 {
        &self.replay
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PlayerActionContent, player_action_content, InGameCommand, in_game_command};

    fn create_test_replay() -> ReplayV1 {
        ReplayV1 {
            metadata: Some(ReplayV1Metadata {
                engine_version: "1.0.0".to_string(),
                game_started_timestamp_ms: 1234567890,
                game: ReplayGame::Snake.into(),
                seed: 42,
                lobby_settings: None,
                players: vec![
                    PlayerIdentity { player_id: "player1".to_string(), is_bot: false },
                    PlayerIdentity { player_id: "player2".to_string(), is_bot: false },
                ],
            }),
            actions: vec![
                PlayerAction {
                    tick: 1,
                    player_index: 0,
                    content: Some(PlayerActionContent {
                        content: Some(player_action_content::Content::Command(InGameCommand {
                            command: Some(in_game_command::Command::Snake(
                                crate::SnakeInGameCommand {
                                    command: Some(crate::proto::snake::snake_in_game_command::Command::Turn(
                                        crate::TurnCommand { direction: 1 }
                                    )),
                                }
                            )),
                        })),
                    }),
                },
                PlayerAction {
                    tick: 2,
                    player_index: 1,
                    content: Some(PlayerActionContent {
                        content: Some(player_action_content::Content::Command(InGameCommand {
                            command: Some(in_game_command::Command::Snake(
                                crate::SnakeInGameCommand {
                                    command: Some(crate::proto::snake::snake_in_game_command::Command::Turn(
                                        crate::TurnCommand { direction: 2 }
                                    )),
                                }
                            )),
                        })),
                    }),
                },
                PlayerAction {
                    tick: 2,
                    player_index: 0,
                    content: Some(PlayerActionContent {
                        content: Some(player_action_content::Content::Command(InGameCommand {
                            command: Some(in_game_command::Command::Snake(
                                crate::SnakeInGameCommand {
                                    command: Some(crate::proto::snake::snake_in_game_command::Command::Turn(
                                        crate::TurnCommand { direction: 3 }
                                    )),
                                }
                            )),
                        })),
                    }),
                },
            ],
        }
    }

    #[test]
    fn test_replay_player_basic() {
        let replay = create_test_replay();
        let player = ReplayPlayer::new(replay);

        assert_eq!(player.engine_version(), "1.0.0");
        assert_eq!(player.seed(), 42);
        assert_eq!(player.players().len(), 2);
        assert_eq!(player.total_actions(), 3);
        assert!(!player.is_finished());
    }

    #[test]
    fn test_replay_player_next_action() {
        let replay = create_test_replay();
        let mut player = ReplayPlayer::new(replay);

        let action1 = player.next_action().unwrap();
        assert_eq!(action1.tick, 1);
        assert_eq!(action1.player_index, 0);

        let action2 = player.next_action().unwrap();
        assert_eq!(action2.tick, 2);
        assert_eq!(action2.player_index, 1);

        let action3 = player.next_action().unwrap();
        assert_eq!(action3.tick, 2);
        assert_eq!(action3.player_index, 0);

        assert!(player.next_action().is_none());
        assert!(player.is_finished());
    }

    #[test]
    fn test_replay_player_actions_for_tick() {
        let replay = create_test_replay();
        let mut player = ReplayPlayer::new(replay);

        let tick1_actions = player.actions_for_tick(1);
        assert_eq!(tick1_actions.len(), 1);

        let tick2_actions = player.actions_for_tick(2);
        assert_eq!(tick2_actions.len(), 2);

        assert!(player.is_finished());
    }

    #[test]
    fn test_replay_player_reset() {
        let replay = create_test_replay();
        let mut player = ReplayPlayer::new(replay);

        player.next_action();
        player.next_action();
        assert_eq!(player.current_action_index(), 2);

        player.reset();
        assert_eq!(player.current_action_index(), 0);
        assert!(!player.is_finished());
    }
}
