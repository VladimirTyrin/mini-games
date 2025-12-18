use common::id_generator::generate_client_id;
use common::menu_service_client::MenuServiceClient;
use common::game_service_client::GameServiceClient;
use common::logger;
use common::log;
use clap::Parser;

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

    let menu_client = MenuServiceClient::connect(config.server_address.clone()).await?;
    let game_client = GameServiceClient::connect(config.server_address.clone()).await?;

    log!("Connected to server");

    Ok(())
}
