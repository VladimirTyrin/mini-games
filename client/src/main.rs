use common::id_generator::generate_client_id;
use common::menu_service_client::MenuServiceClient;
use common::game_service_client::GameServiceClient;
use common::{MenuClientMessage, ConnectRequest, DisconnectRequest, logger, log};
use clap::Parser;
use tokio::time::{sleep, Duration};

#[derive(Parser)]
#[command(name = "snake_game_client")]
struct Args {
    #[arg(long)]
    use_log_prefix: bool,
}

struct ClientConfig {
    server_address: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_address: "http://[::1]:5001".to_string(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let client_id = generate_client_id();

    let prefix = if args.use_log_prefix {
        Some(client_id.clone())
    } else {
        None
    };
    logger::init_logger(prefix);

    let config = ClientConfig::default();
    log!("Snake Game Client: {}", client_id);

    let mut menu_client = MenuServiceClient::connect(config.server_address.clone()).await?;
    let _game_client = GameServiceClient::connect(config.server_address.clone()).await?;

    log!("Connected to server");

    let (tx, rx) = tokio::sync::mpsc::channel(128);

    let menu_stream = menu_client.menu_stream(tokio_stream::wrappers::ReceiverStream::new(rx)).await?;
    let mut response_stream = menu_stream.into_inner();

    tx.send(MenuClientMessage {
        client_id: client_id.clone(),
        message: Some(common::menu_client_message::Message::Connect(ConnectRequest {})),
    }).await?;

    log!("Sent connect message to menu service");

    sleep(Duration::from_secs(2)).await;

    tx.send(MenuClientMessage {
        client_id: client_id.clone(),
        message: Some(common::menu_client_message::Message::Disconnect(DisconnectRequest {})),
    }).await?;

    log!("Sent disconnect message to menu service");

    drop(tx);

    while let Some(result) = response_stream.message().await? {
        if let Some(msg) = result.message {
            match msg {
                common::menu_server_message::Message::Error(err) => {
                    log!("Server error: {}", err.message);
                }
                _ => {}
            }
        }
    }

    Ok(())
}
