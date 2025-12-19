use common::id_generator::generate_client_id;
use common::menu_service_client::MenuServiceClient;
use common::{
    MenuClientMessage, ConnectRequest, DisconnectRequest,
    ListLobbiesRequest, CreateLobbyRequest, JoinLobbyRequest, LeaveLobbyRequest,
    MarkReadyRequest, LobbyInfo, LobbyDetails, LobbySettings,
};
use eframe::egui;
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
enum ClientCommand {
    ListLobbies,
    CreateLobby { name: String, max_players: u32 },
    JoinLobby { lobby_id: String },
    LeaveLobby,
    MarkReady { ready: bool },
}

#[derive(Debug, Clone)]
enum AppState {
    LobbyList {
        lobbies: Vec<LobbyInfo>,
    },
    InLobby {
        details: LobbyDetails,
        event_log: Vec<String>,
    },
}

struct SharedState {
    state: Arc<Mutex<AppState>>,
    error: Arc<Mutex<Option<String>>>,
}

impl SharedState {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AppState::LobbyList { lobbies: vec![] })),
            error: Arc::new(Mutex::new(None)),
        }
    }

    fn set_state(&self, state: AppState) {
        *self.state.lock().unwrap() = state;
    }

    fn get_state(&self) -> AppState {
        self.state.lock().unwrap().clone()
    }

    fn add_event(&self, event: String) {
        let mut state = self.state.lock().unwrap();
        if let AppState::InLobby { event_log, .. } = &mut *state {
            event_log.push(event);
        }
    }

    fn set_error(&self, error: String) {
        *self.error.lock().unwrap() = Some(error);
    }

    fn get_error(&self) -> Option<String> {
        self.error.lock().unwrap().clone()
    }

    fn clear_error(&self) {
        *self.error.lock().unwrap() = None;
    }
}

impl Clone for SharedState {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            error: Arc::clone(&self.error),
        }
    }
}

struct MenuApp {
    client_id: String,
    shared_state: SharedState,
    command_tx: mpsc::UnboundedSender<ClientCommand>,
    create_lobby_dialog: bool,
    lobby_name_input: String,
    max_players_input: String,
}

impl MenuApp {
    fn new(
        client_id: String,
        shared_state: SharedState,
        command_tx: mpsc::UnboundedSender<ClientCommand>,
    ) -> Self {
        Self {
            client_id,
            shared_state,
            command_tx,
            create_lobby_dialog: false,
            lobby_name_input: String::new(),
            max_players_input: "4".to_string(),
        }
    }

    fn render_lobby_list(&mut self, ui: &mut egui::Ui, lobbies: &[LobbyInfo]) {
        ui.heading("Snake Game - Lobby List");
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("🔄 Update").clicked() {
                let _ = self.command_tx.send(ClientCommand::ListLobbies);
            }

            if ui.button("➕ Create Lobby").clicked() {
                self.create_lobby_dialog = true;
                self.lobby_name_input.clear();
                self.max_players_input = "4".to_string();
            }
        });

        ui.separator();

        if lobbies.is_empty() {
            ui.label("No lobbies available. Create one!");
        } else {
            egui::ScrollArea::vertical()
                .id_salt("lobby_list_scroll")
                .show(ui, |ui| {
                for lobby in lobbies {
                    let can_join = lobby.current_players < lobby.max_players;

                    let (rect, inner_response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 60.0),
                        egui::Sense::click(),
                    );

                    let button_clicked = ui.allocate_new_ui(
                        egui::UiBuilder::new().max_rect(rect),
                        |ui| {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(format!("📋 {}", lobby.lobby_name));
                                        ui.label(format!(
                                            "👥 Players: {}/{}",
                                            lobby.current_players, lobby.max_players
                                        ));
                                    });

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.add_enabled_ui(can_join, |ui| {
                                            ui.button("Join").clicked()
                                        })
                                    })
                                })
                            })
                        }
                    ).inner.inner.inner.inner.inner;

                    let double_clicked = inner_response.double_clicked() && can_join;

                    if button_clicked || double_clicked {
                        let _ = self.command_tx.send(ClientCommand::JoinLobby {
                            lobby_id: lobby.lobby_id.clone(),
                        });
                    }
                }
            });
        }
    }

    fn render_create_lobby_dialog(&mut self, ctx: &egui::Context) {
        let mut close_dialog = false;

        egui::Window::new("Create Lobby")
            .open(&mut self.create_lobby_dialog)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label("Lobby Name:");
                ui.text_edit_singleline(&mut self.lobby_name_input);

                ui.label("Max Players:");
                ui.text_edit_singleline(&mut self.max_players_input);

                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        if let Ok(max_players) = self.max_players_input.parse::<u32>() {
                            if !self.lobby_name_input.is_empty() && max_players >= 2 && max_players <= 10 {
                                let _ = self.command_tx.send(ClientCommand::CreateLobby {
                                    name: self.lobby_name_input.clone(),
                                    max_players,
                                });
                                close_dialog = true;
                            } else {
                                self.shared_state.set_error(
                                    "Invalid input: Name required, 2-10 players".to_string()
                                );
                            }
                        } else {
                            self.shared_state.set_error("Invalid max players number".to_string());
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        close_dialog = true;
                    }
                });
            });

        if close_dialog {
            self.create_lobby_dialog = false;
        }
    }

    fn render_in_lobby(&mut self, ui: &mut egui::Ui, details: &LobbyDetails, event_log: &[String]) {
        ui.heading(format!("Lobby: {}", details.lobby_name));
        ui.separator();

        ui.label(format!("Lobby ID: {}", details.lobby_id));
        ui.label(format!("Players: {}/{}", details.players.len(), details.max_players));

        ui.separator();
        ui.heading("Players:");

        for player in &details.players {
            ui.horizontal(|ui| {
                let is_self = player.client_id == self.client_id;
                let player_display = if is_self {
                    format!("👤 {} (You)", player.client_id)
                } else {
                    format!("👤 {}", player.client_id)
                };

                ui.label(player_display);

                if player.ready {
                    ui.label("✅ Ready");
                } else {
                    ui.label("⏳ Not Ready");
                }
            });
        }

        ui.separator();

        let current_ready = details.players
            .iter()
            .find(|p| p.client_id == self.client_id)
            .map(|p| p.ready)
            .unwrap_or(false);

        ui.horizontal(|ui| {
            let button_text = if current_ready { "Mark Not Ready" } else { "Mark Ready" };
            if ui.button(button_text).clicked() {
                let _ = self.command_tx.send(ClientCommand::MarkReady {
                    ready: !current_ready,
                });
            }

            if ui.button("🚪 Leave Lobby").clicked() {
                let _ = self.command_tx.send(ClientCommand::LeaveLobby);
            }
        });

        ui.separator();
        ui.heading("Events:");

        egui::ScrollArea::vertical()
            .id_salt("events_scroll")
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for event in event_log {
                    ui.label(event);
                }
            });
    }
}

impl eframe::App for MenuApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(error) = self.shared_state.get_error() {
            egui::Window::new("Error")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(&error);
                    if ui.button("OK").clicked() {
                        self.shared_state.clear_error();
                    }
                });
        }

        if self.create_lobby_dialog {
            self.render_create_lobby_dialog(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.shared_state.get_state() {
                AppState::LobbyList { lobbies } => {
                    self.render_lobby_list(ui, &lobbies);
                }
                AppState::InLobby { details, event_log } => {
                    self.render_in_lobby(ui, &details, &event_log);
                }
            }
        });

        ctx.request_repaint();
    }
}

async fn grpc_client_task(
    client_id: String,
    server_address: String,
    shared_state: SharedState,
    mut command_rx: mpsc::UnboundedReceiver<ClientCommand>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut menu_client = MenuServiceClient::connect(server_address).await?;

    let (tx, rx) = mpsc::channel(128);

    let menu_stream = menu_client
        .menu_stream(tokio_stream::wrappers::ReceiverStream::new(rx))
        .await?;
    let mut response_stream = menu_stream.into_inner();

    tx.send(MenuClientMessage {
        client_id: client_id.clone(),
        message: Some(common::menu_client_message::Message::Connect(ConnectRequest {})),
    })
    .await?;

    tx.send(MenuClientMessage {
        client_id: client_id.clone(),
        message: Some(common::menu_client_message::Message::ListLobbies(ListLobbiesRequest {})),
    })
    .await?;

    loop {
        tokio::select! {
            Some(command) = command_rx.recv() => {
                let message = match command {
                    ClientCommand::ListLobbies => {
                        Some(common::menu_client_message::Message::ListLobbies(ListLobbiesRequest {}))
                    }
                    ClientCommand::CreateLobby { name, max_players } => {
                        Some(common::menu_client_message::Message::CreateLobby(CreateLobbyRequest {
                            lobby_name: name,
                            max_players,
                            settings: Some(LobbySettings {}),
                        }))
                    }
                    ClientCommand::JoinLobby { lobby_id } => {
                        Some(common::menu_client_message::Message::JoinLobby(JoinLobbyRequest {
                            lobby_id,
                        }))
                    }
                    ClientCommand::LeaveLobby => {
                        Some(common::menu_client_message::Message::LeaveLobby(LeaveLobbyRequest {}))
                    }
                    ClientCommand::MarkReady { ready } => {
                        Some(common::menu_client_message::Message::MarkReady(MarkReadyRequest {
                            ready,
                        }))
                    }
                };

                if let Some(msg) = message {
                    if tx.send(MenuClientMessage {
                        client_id: client_id.clone(),
                        message: Some(msg),
                    }).await.is_err() {
                        break;
                    }
                }
            }

            result = response_stream.message() => {
                match result {
                    Ok(Some(server_msg)) => {
                        if let Some(msg) = server_msg.message {
                            match msg {
                                common::menu_server_message::Message::LobbyList(list) => {
                                    shared_state.set_state(AppState::LobbyList {
                                        lobbies: list.lobbies,
                                    });
                                }
                                common::menu_server_message::Message::LobbyUpdate(update) => {
                                    if let Some(details) = update.lobby {
                                        let current_state = shared_state.get_state();
                                        match current_state {
                                            AppState::InLobby { event_log, .. } => {
                                                shared_state.set_state(AppState::InLobby {
                                                    details,
                                                    event_log,
                                                });
                                            }
                                            _ => {
                                                shared_state.set_state(AppState::InLobby {
                                                    details,
                                                    event_log: vec![],
                                                });
                                            }
                                        }
                                    }
                                }
                                common::menu_server_message::Message::PlayerJoined(notification) => {
                                    shared_state.add_event(format!("{} joined", notification.client_id));
                                }
                                common::menu_server_message::Message::PlayerLeft(notification) => {
                                    shared_state.add_event(format!("{} left", notification.client_id));
                                }
                                common::menu_server_message::Message::PlayerReady(notification) => {
                                    if notification.ready {
                                        shared_state.add_event(format!("{} is ready", notification.client_id));
                                    } else {
                                        shared_state.add_event(format!("{} is not ready anymore", notification.client_id));
                                    }
                                }
                                common::menu_server_message::Message::LobbyListUpdate(_) => {
                                    let current_state = shared_state.get_state();
                                    if matches!(current_state, AppState::LobbyList { .. }) {
                                        if tx.send(MenuClientMessage {
                                            client_id: client_id.clone(),
                                            message: Some(common::menu_client_message::Message::ListLobbies(
                                                ListLobbiesRequest {}
                                            )),
                                        }).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                                common::menu_server_message::Message::Error(err) => {
                                    shared_state.set_error(err.message);
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        shared_state.set_error(format!("Connection error: {}", e));
                        break;
                    }
                }
            }
        }
    }

    let _ = tx.send(MenuClientMessage {
        client_id: client_id.clone(),
        message: Some(common::menu_client_message::Message::Disconnect(DisconnectRequest {})),
    }).await;

    Ok(())
}

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
