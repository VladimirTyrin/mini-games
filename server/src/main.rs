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
    pub mod puzzle2048 {
        tonic::include_proto!("puzzle2048");
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
    pub use puzzle2048::{
        Puzzle2048LobbySettings, Puzzle2048InGameCommand, Puzzle2048GameState,
        Puzzle2048GameEndReason, Puzzle2048GameEndInfo,
    };
    pub use replay::{
        Game as ReplayGame, PlayerAction, PlayerActionContent, PlayerDisconnected,
        ReplayV1, ReplayV1Metadata, ReplayV1Header,
    };
    pub use replay::player_action_content;
}

pub use proto::*;
pub use identifiers::*;

pub mod id_generator;
pub mod logger;
pub mod identifiers;
pub mod config;
pub mod version;
pub mod validate_lobby_settings;
pub mod lobby;
pub mod replay;
pub mod games;

mod broadcaster;
mod cleanup_task;
mod server_config;
mod game_session_manager;
mod grpc_service;
mod lobby_manager;
mod message_handler;
mod replay_session;
mod web_server;
mod ws_handler;

use std::path::PathBuf;

use broadcaster::Broadcaster;
use clap::Parser;
use game_session_manager::GameSessionManager;
use grpc_service::GrpcService;
use lobby_manager::LobbyManager;
use tonic::transport::Server;

#[derive(Parser)]
#[command(name = "mini_games_server")]
struct Args {
    #[arg(long)]
    use_log_prefix: bool,

    #[arg(long, default_value = "./web-client/dist")]
    static_files_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let prefix = if args.use_log_prefix {
        Some("Server".to_string())
    } else {
        None
    };
    logger::init_logger(prefix);

    let addr = "0.0.0.0:5001".parse()?;
    let lobby_manager = LobbyManager::new();
    let broadcaster = Broadcaster::new();
    let session_manager = GameSessionManager::new(broadcaster.clone(), lobby_manager.clone());

    let service = GrpcService::new(lobby_manager.clone(), broadcaster.clone(), session_manager.clone());

    let cleanup_task = cleanup_task::CleanupTask::new(
        lobby_manager.clone(),
        broadcaster.clone(),
        server_config::CLEANUP_CHECK_INTERVAL,
        server_config::INACTIVITY_TIMEOUT,
    );
    tokio::spawn(async move {
        cleanup_task.run().await;
    });

    log!("Mini Games Server - gRPC on {}, Web/WebSocket on 0.0.0.0:5000", addr);

    let broadcaster_clone = broadcaster.clone();
    let shutdown_signal = async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");

        log!("Shutdown signal received, notifying clients...");

        let shutdown_msg = ServerMessage {
            message: Some(server_message::Message::Shutdown(
                ServerShuttingDownNotification {
                    message: "Server is shutting down".to_string(),
                }
            )),
        };

        broadcaster_clone.broadcast_to_all(shutdown_msg).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    };

    let grpc_server = Server::builder()
        .add_service(proto::game_service::game_service_server::GameServiceServer::new(service))
        .serve_with_shutdown(addr, shutdown_signal);

    let web_server = web_server::run_web_server(
        lobby_manager,
        broadcaster,
        session_manager,
        args.static_files_path,
    );

    tokio::select! {
        result = grpc_server => {
            if let Err(e) = result {
                log!("gRPC server error: {}", e);
            }
        }
        () = web_server => {
            log!("Web server stopped");
        }
    }

    log!("Server shut down gracefully");

    Ok(())
}
