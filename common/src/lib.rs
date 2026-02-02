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
    pub mod numbers_match {
        tonic::include_proto!("numbers_match");
    }
    pub mod stack_attack {
        tonic::include_proto!("stack_attack");
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
    pub use numbers_match::{
        NumbersMatchLobbySettings, NumbersMatchInGameCommand, NumbersMatchGameState,
        NumbersMatchGameEndReason, NumbersMatchGameEndInfo,
        HintMode as ProtoHintMode,
    };
    pub use stack_attack::{
        StackAttackLobbySettings, StackAttackInGameCommand, StackAttackGameState,
        StackAttackGameEndReason, StackAttackGameEndInfo,
    };
    pub use replay::{Game as ReplayGame, PlayerAction, PlayerActionContent, PlayerDisconnected, ReplayV1, ReplayV1Metadata, ReplayV1Header};
    pub use replay::player_action_content;
}

pub mod id_generator;
pub mod logger;
pub mod identifiers;
pub mod config;
pub mod version;
pub mod validate_lobby_settings;
pub mod lobby;
pub mod replay;
pub mod games;

pub use proto::*;
pub use identifiers::*;