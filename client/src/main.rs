mod state;
mod client;
mod ui;

use common::id_generator::generate_client_id;
use eframe::egui;
use tokio::sync::mpsc;

use state::SharedState;
use client::grpc_client_task;
use ui::MenuApp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_id = generate_client_id();
    let server_address = "http://[::1]:5001".to_string();

    let shared_state = SharedState::new();
    let (command_tx, command_rx) = mpsc::unbounded_channel();

    let client_id_clone = client_id.clone();
    let server_address_clone = server_address.clone();
    let shared_state_clone = shared_state.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = grpc_client_task(
                client_id_clone,
                server_address_clone,
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
            )))
        }),
    )?;

    Ok(())
}
