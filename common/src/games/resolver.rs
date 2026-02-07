use crate::{ClientId, GameOverNotification, InGameCommand, in_game_command};
use crate::games::{
    GameBroadcaster, GameSession, GameSessionConfig, LobbySettings, ReplayMode,
    numbers_match::NumbersMatchSession,
    puzzle2048::Puzzle2048Session,
    snake::{DeathReason, SnakeSession},
    stack_attack::StackAttackSession,
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
            GameSession::NumbersMatch(state) => {
                NumbersMatchSession::run(&config, &state, &broadcaster).await
            }
            GameSession::StackAttack(state) => {
                StackAttackSession::run(config, state, broadcaster).await
            }
            GameSession::Puzzle2048(state) => {
                Puzzle2048Session::run(&config, &state, &broadcaster).await
            }
        }
    }

    pub async fn handle_command(
        session: &GameSession,
        client_id: &ClientId,
        command: InGameCommand,
    ) {
        match (session, command.command) {
            (GameSession::Snake(state), Some(in_game_command::Command::Snake(cmd))) => {
                SnakeSession::handle_command(state, client_id, &cmd).await;
            }
            (GameSession::TicTacToe(state), Some(in_game_command::Command::Tictactoe(cmd))) => {
                TicTacToeSession::handle_command(state, client_id, &cmd).await;
            }
            (GameSession::NumbersMatch(state), Some(in_game_command::Command::NumbersMatch(cmd))) => {
                NumbersMatchSession::handle_command(state, client_id, cmd).await;
            }
            (GameSession::StackAttack(state), Some(in_game_command::Command::StackAttack(cmd))) => {
                StackAttackSession::handle_command(state, client_id, cmd).await;
            }
            (GameSession::Puzzle2048(state), Some(in_game_command::Command::Puzzle2048(cmd))) => {
                Puzzle2048Session::handle_command(state, client_id, cmd).await;
            }
            _ => {}
        }
    }

    pub async fn handle_player_disconnect(session: &GameSession, client_id: &ClientId) {
        session.record_disconnect(client_id).await;

        match session {
            GameSession::Snake(state) => {
                SnakeSession::handle_kill_snake(state, client_id, DeathReason::PlayerDisconnected).await;
            }
            GameSession::TicTacToe(state) => {
                TicTacToeSession::handle_player_disconnect(state, client_id).await;
            }
            GameSession::NumbersMatch(state) => {
                NumbersMatchSession::handle_player_disconnect(state).await;
            }
            GameSession::StackAttack(state) => {
                StackAttackSession::handle_player_disconnect(state).await;
            }
            GameSession::Puzzle2048(state) => {
                Puzzle2048Session::handle_player_disconnect(state).await;
            }
        }
    }
}
