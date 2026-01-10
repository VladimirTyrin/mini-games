mod command_sender;
mod config;
mod constants;
mod file_association;
mod grpc_client;
mod offline;
mod replay_playback;
mod state;
mod ui;

pub use command_sender::CommandSender;

use clap::Parser;
use common::{id_generator::generate_client_id, log};
use common::logger::init_logger;
use eframe::egui;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::get_config_manager;
use grpc_client::grpc_client_task;
use state::SharedState;
use ui::{MenuApp, prompt_for_username};

#[derive(Parser)]
#[command(name = "mini_games_client")]
struct Args {
    #[arg(long)]
    use_log_prefix: bool,

    #[arg(long)]
    server_address: Option<String>,

    #[arg(long)]
    random_client_id: bool,

    /// Replay file to open on startup
    #[arg(value_name = "REPLAY_FILE")]
    replay_file: Option<std::path::PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let config_manager = get_config_manager();
    let mut config = config_manager.get_config()?;

    let client_id = if args.random_client_id {
        generate_client_id()
    } else if let Some(ref id) = config.client_id {
        id.clone()
    } else {
        let username = prompt_for_username()
            .ok_or("Username input was cancelled")?;
        config.client_id = Some(username.clone());
        config_manager.set_config(&config)?;
        username
    };

    let prefix = if args.use_log_prefix {
        Some(client_id.clone())
    } else {
        None
    };
    init_logger(prefix);

    if !config.file_association_registered
        && let Ok(exe_path) = std::env::current_exe()
        && file_association::register_file_association(&exe_path).is_ok()
    {
        config.file_association_registered = true;
        if let Err(e) = config_manager.set_config(&config) {
            log!("Failed to save file association config: {}", e);
        }
    }

    let startup_replay_file = args.replay_file.clone();

    let shared_state = SharedState::new();
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let command_sender = CommandSender::Grpc(command_tx);

    let client_id_clone = client_id.clone();
    let server_address = args.server_address.or(config.server.address.clone());
    let shared_state_clone = shared_state.clone();

    if server_address.is_none() {
        shared_state.set_connection_failed(true);
    }

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
            .with_min_inner_size([400.0, 500.0])
            .with_title(format!("Snake Game - {}", client_id)),
        ..Default::default()
    };

    let disconnect_timeout = Duration::from_millis(config.server.disconnect_timeout_ms as u64);

    eframe::run_native(
        "Mini Games Client",
        options,
        Box::new(move |_cc| {
            let mut app = MenuApp::new(
                client_id,
                shared_state,
                command_sender,
                disconnect_timeout,
                config_manager
            );

            if let Some(replay_path) = startup_replay_file {
                app.open_replay_file(&replay_path);
            }

            Ok(Box::new(app))
        }),
    )?;

    Ok(())
}
