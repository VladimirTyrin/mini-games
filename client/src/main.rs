mod state;
mod client;
mod menu_ui;
mod sprites;
mod game_ui;
mod config;
mod colors;

use clap::Parser;
use common::id_generator::generate_client_id;
use eframe::egui;
use tokio::sync::mpsc;
use std::time::Duration;

use client::grpc_client_task;
use common::logger::init_logger;
use state::SharedState;
use menu_ui::MenuApp;
use crate::config::get_config_manager;

#[derive(Parser)]
#[command(name = "snake_game_client")]
struct Args {
    #[arg(long)]
    use_log_prefix: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_manager = get_config_manager();
    let config = config_manager.get_config()?;

    let client_id = config.client_id.unwrap_or_else(|| generate_client_id());

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
    let server_address = config.server.address.clone();
    let shared_state_clone = shared_state.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config_manager = get_config_manager();
            if let Err(e) = grpc_client_task(
                client_id_clone,
                server_address,
                shared_state_clone,
                command_rx,
                config_manager,
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

    let disconnect_timeout = Duration::from_millis(config.server.disconnect_timeout_ms as u64);

    eframe::run_native(
        "Snake Game Client",
        options,
        Box::new(|_cc| {
            Ok(Box::new(MenuApp::new(
                client_id,
                shared_state,
                command_tx,
                disconnect_timeout,
                config_manager
            )))
        }),
    )?;

    Ok(())
}
