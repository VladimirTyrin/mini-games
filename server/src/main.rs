mod connection_tracker;
mod menu_service;
mod game_service;
mod lobby_manager;

use tonic::transport::Server;
use common::{
    menu_service_server::MenuServiceServer,
    game_service_server::GameServiceServer,
    logger,
    log,
};
use clap::Parser;
use connection_tracker::ConnectionTracker;
use menu_service::MenuServiceImpl;
use game_service::GameServiceImpl;
use lobby_manager::LobbyManager;

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

    let menu_service = MenuServiceImpl::new(tracker.clone(), lobby_manager);
    let game_service = GameServiceImpl::new(tracker);

    log!("Snake Game Server listening on {}", addr);

    Server::builder()
        .add_service(MenuServiceServer::new(menu_service))
        .add_service(GameServiceServer::new(game_service))
        .serve(addr)
        .await?;

    Ok(())
}
