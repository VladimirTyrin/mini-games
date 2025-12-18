use common::id_generator::generate_client_id;
use common::menu_service_client::MenuServiceClient;
use common::game_service_client::GameServiceClient;

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
    let config = ClientConfig::default();
    let client_id = generate_client_id();
    println!("Snake Game Client: {}", client_id);

    let menu_client = MenuServiceClient::connect(config.server_address.clone()).await?;
    let game_client = GameServiceClient::connect(config.server_address.clone()).await?;

    println!("Connected to server");

    Ok(())
}
