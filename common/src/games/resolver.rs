use crate::{ClientId, GameOverNotification, InGameCommand, in_game_command};
use crate::games::{
    GameBroadcaster, GameSession, GameSessionConfig, LobbySettings, ReplayMode,
    snake::SnakeSession,
    tictactoe::TicTacToeSession,
};

pub struct GameResolver;

impl GameResolver {
    pub fn validate_player_count(
        settings: &impl LobbySettings,
        player_count: usize,
    ) -> Result<(), String> {
        settings.validate_player_count(player_count)
    }

    pub fn create_session(
        config: &GameSessionConfig,
        settings: &impl LobbySettings,
        seed: u64,
        replay_mode: ReplayMode,
    ) -> Result<GameSession, String> {
        settings.create_session(config, seed, replay_mode)
    }

    pub async fn run(
        config: GameSessionConfig,
        session: GameSession,
        broadcaster: impl GameBroadcaster,
    ) -> GameOverNotification {
        match session {
            GameSession::Snake(state) => SnakeSession::run(config, state, broadcaster).await,
            GameSession::TicTacToe(state) => TicTacToeSession::run(config, state, broadcaster).await,
        }
    }

    pub async fn handle_command(
        session: &GameSession,
        client_id: &ClientId,
        command: InGameCommand,
    ) {
        match (session, &command.command) {
            (GameSession::Snake(state), Some(in_game_command::Command::Snake(cmd))) => {
                SnakeSession::handle_command(state, client_id, cmd).await;
            }
            (GameSession::TicTacToe(state), Some(in_game_command::Command::Tictactoe(cmd))) => {
                TicTacToeSession::handle_command(state, client_id, cmd).await;
            }
            _ => {}
        }
    }
}
