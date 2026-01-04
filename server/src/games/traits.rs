use common::{PlayerId, BotId, PlayerIdentity, ScoreEntry};

#[allow(dead_code)]
pub trait GameLogic: Send + 'static {
    type Command: Clone + Send;

    fn update(&mut self) -> GameTickResult;

    fn handle_command(
        &mut self,
        player_id: &PlayerId,
        command: Self::Command,
    ) -> Result<(), String>;

    fn is_game_over(&self) -> bool;

    fn get_winner(&self) -> Option<PlayerIdentity>;

    fn get_scores(&self) -> Vec<ScoreEntry>;

    fn calculate_bot_move(
        &self,
        bot_id: &BotId,
    ) -> Option<Self::Command>;
}

#[allow(dead_code)]
pub enum GameTickResult {
    Continue,
    GameOver {
        winner: Option<PlayerIdentity>,
        reason: GameEndReason,
    },
}

#[allow(dead_code)]
pub enum GameEndReason {
    Snake(common::SnakeGameEndReason),
    TicTacToe(common::TicTacToeGameEndReason),
}
