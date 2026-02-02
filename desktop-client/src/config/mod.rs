mod main_config;
mod game_type;
mod numbers_match_lobby_config;
mod replay_config;
mod server_config;
mod snake_lobby_config;
mod tictactoe_lobby_config;

pub(crate) use common::config::{ConfigManager, FileContentConfigProvider, YamlConfigSerializer};

pub use main_config::{get_config_manager, Config};
pub use game_type::GameType;
pub use numbers_match_lobby_config::NumbersMatchLobbyConfig;
pub use replay_config::ReplayConfig;
pub use server_config::ServerConfig;
pub use snake_lobby_config::SnakeLobbyConfig;
pub use tictactoe_lobby_config::TicTacToeLobbyConfig;
