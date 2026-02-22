mod board;
mod bot_controller;
mod game_state;
pub(crate) mod replay;
mod session;
mod settings;
mod types;
mod validate;
mod win_detector;

pub use board::get_available_moves;
pub use bot_controller::{BotInput, calculate_minimax_move, calculate_move};
pub use game_state::TicTacToeGameState;
pub use session::{TicTacToeSession, TicTacToeSessionState};
pub use settings::TicTacToeSessionSettings;
pub use types::{FirstPlayerMode, GameStatus, Mark, Position, WinningLine};
pub use win_detector::{check_win, check_win_with_line};
