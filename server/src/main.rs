mod lobby_manager;
mod broadcaster;
mod service;
mod games;
mod game_session_manager;

use tonic::transport::Server;
use common::{
    proto::game_service::game_service_server::GameServiceServer,
    logger,
    log,
    ServerMessage, server_message,
    ServerShuttingDownNotification,
};
use clap::Parser;
use service::GameServiceImpl;
use lobby_manager::LobbyManager;
use broadcaster::Broadcaster;
use game_session_manager::GameSessionManager;

#[derive(Parser)]
#[command(name = "mini_games_server")]
struct Args {
    #[arg(long)]
    use_log_prefix: bool,
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

    let service = GameServiceImpl::new(lobby_manager, broadcaster.clone(), session_manager);

    log!("Mini Games Server listening on {}", addr);

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

    Server::builder()
        .add_service(GameServiceServer::new(service))
        .serve_with_shutdown(addr, shutdown_signal)
        .await?;

    log!("Server shut down gracefully");

    Ok(())
}
