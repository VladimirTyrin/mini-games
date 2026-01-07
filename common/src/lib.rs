pub mod proto {
    pub mod game_service {
        tonic::include_proto!("game_service");
    }
    pub mod snake {
        tonic::include_proto!("snake");
    }
    pub mod tictactoe {
        tonic::include_proto!("tictactoe");
    }
    pub mod replay {
        tonic::include_proto!("replay");
    }

    pub use game_service::*;
    pub use game_service::{
        lobby_details, add_bot_request, lobby_settings,
        in_game_command, game_state_update, game_over_notification,
    };
    pub use snake::{
        Direction, SnakeLobbySettings, SnakeBotType, WallCollisionMode,
        DeadSnakeBehavior, SnakeGameEndReason, SnakeInGameCommand, TurnCommand,
        SnakeGameState, Snake, Position as SnakePosition,
    };
    pub use tictactoe::{
        TicTacToeLobbySettings, TicTacToeBotType, FirstPlayerMode,
        TicTacToeGameEndReason, TicTacToeInGameCommand, PlaceMarkCommand,
        TicTacToeGameState, CellMark, MarkType, GameStatus,
    };
    pub use replay::{Game as ReplayGame, PlayerAction, PlayerActionContent, PlayerDisconnected, ReplayV1};
    pub use replay::player_action_content;
}

pub mod id_generator;
pub mod logger;
pub mod identifiers;
pub mod config;
pub mod version;
pub mod validation;
pub mod engine;
pub mod lobby;
pub mod replay;

pub use proto::*;
pub use identifiers::*;