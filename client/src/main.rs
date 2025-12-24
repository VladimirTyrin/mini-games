mod state;
mod client;
mod ui;
mod settings;
mod game_render;
mod game_ui;
mod config;

use clap::Parser;
use common::id_generator::generate_client_id;
use eframe::egui;
use tokio::sync::mpsc;

use client::grpc_client_task;
use common::logger::init_logger;
use settings::ClientSettings;
use state::SharedState;
use ui::MenuApp;

#[derive(Parser)]
#[command(name = "snake_game_client")]
struct Args {
    #[arg(long)]
    use_log_prefix: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = ClientSettings::default();
    let client_id = generate_client_id();

    let args = Args::parse();

    let prefix = if args.use_log_prefix {
        Some(client_id.clone())
    } else {
        None
    };
    init_logger(prefix);

    let shared_state = SharedState::new();
    let (command_tx, command_rx) = mpsc::unbounded_channel();

    let client_id_clone = client_id.clone();
    let server_address = settings.server_address.clone();
    let shared_state_clone = shared_state.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = grpc_client_task(
                client_id_clone,
                server_address,
                shared_state_clone,
                command_rx,
            ).await {
                eprintln!("gRPC client error: {}", e);
            }
        });
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 700.0])
            .with_title(format!("Snake Game - {}", client_id)),
        ..Default::default()
    };

    eframe::run_native(
        "Snake Game Client",
        options,
        Box::new(|_cc| {
            Ok(Box::new(MenuApp::new(
                client_id,
                shared_state,
                command_tx,
                settings.disconnect_timeout,
            )))
        }),
    )?;

    Ok(())
}
