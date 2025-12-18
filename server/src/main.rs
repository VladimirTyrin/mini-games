use tonic::{transport::Server, Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use common::{
    menu_service_server::{MenuService, MenuServiceServer},
    game_service_server::{GameService, GameServiceServer},
    MenuClientMessage, MenuServerMessage,
    GameClientMessage, GameServerMessage,
};

#[derive(Debug, Default)]
pub struct MenuServiceImpl {}

#[tonic::async_trait]
impl MenuService for MenuServiceImpl {
    type MenuStreamStream = ReceiverStream<Result<MenuServerMessage, Status>>;

    async fn menu_stream(
        &self,
        request: Request<tonic::Streaming<MenuClientMessage>>,
    ) -> Result<Response<Self::MenuStreamStream>, Status> {
        let (_tx, rx) = tokio::sync::mpsc::channel(128);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[derive(Debug, Default)]
pub struct GameServiceImpl {}

#[tonic::async_trait]
impl GameService for GameServiceImpl {
    type GameStreamStream = ReceiverStream<Result<GameServerMessage, Status>>;

    async fn game_stream(
        &self,
        request: Request<tonic::Streaming<GameClientMessage>>,
    ) -> Result<Response<Self::GameStreamStream>, Status> {
        let (_tx, rx) = tokio::sync::mpsc::channel(128);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:5001".parse()?;
    let menu_service = MenuServiceImpl::default();
    let game_service = GameServiceImpl::default();

    println!("Snake Game Server listening on {}", addr);

    Server::builder()
        .add_service(MenuServiceServer::new(menu_service))
        .add_service(GameServiceServer::new(game_service))
        .serve(addr)
        .await?;

    Ok(())
}
