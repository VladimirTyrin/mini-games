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

    pub use game_service::*;
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
}

pub mod id_generator;
pub mod logger;
pub mod identifiers;
pub mod config;
pub mod version;
pub mod validation;
pub mod engine;
pub mod lobby;

pub use proto::*;
pub use identifiers::*;