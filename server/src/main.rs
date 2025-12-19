mod connection_tracker;
mod menu_service;
mod game_service;
mod lobby_manager;
mod broadcaster;

use tonic::transport::Server;
use common::{
    menu_service_server::MenuServiceServer,
    game_service_server::GameServiceServer,
    logger,
    log,
    MenuServerMessage,
    ServerShuttingDownNotification,
};
use clap::Parser;
use connection_tracker::ConnectionTracker;
use menu_service::MenuServiceImpl;
use game_service::GameServiceImpl;
use lobby_manager::LobbyManager;
use broadcaster::ClientBroadcaster;

#[derive(Parser)]
#[command(name = "snake_game_server")]
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

    let addr = "[::1]:5001".parse()?;
    let tracker = ConnectionTracker::new();
    let lobby_manager = LobbyManager::new();
    let broadcaster = ClientBroadcaster::new();

    let menu_service = MenuServiceImpl::new(tracker.clone(), lobby_manager, broadcaster.clone());
    let game_service = GameServiceImpl::new(tracker);

    log!("Snake Game Server listening on {}", addr);

    let broadcaster_clone = broadcaster.clone();
    let shutdown_signal = async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");

        log!("Shutdown signal received, notifying clients...");

        let shutdown_msg = MenuServerMessage {
            message: Some(common::menu_server_message::Message::ServerShuttingDown(
                ServerShuttingDownNotification {
                    message: "Server is shutting down".to_string(),
                }
            )),
        };

        broadcaster_clone.broadcast_to_all(shutdown_msg).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    };

    Server::builder()
        .add_service(MenuServiceServer::new(menu_service))
        .add_service(GameServiceServer::new(game_service))
        .serve_with_shutdown(addr, shutdown_signal)
        .await?;

    log!("Server shut down gracefully");

    Ok(())
}
