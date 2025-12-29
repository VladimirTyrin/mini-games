use crate::config::{Config, LobbyConfig};
use crate::game_ui::GameUi;
use crate::state::{AppState, MenuCommand, ClientCommand, SharedState};
use common::config::{ConfigManager, FileContentConfigProvider, YamlConfigSerializer};
use common::WallCollisionMode;
use common::{LobbyDetails, LobbyInfo};
use eframe::egui;
use tokio::sync::mpsc;

type ClientConfigManager = ConfigManager<FileContentConfigProvider, Config, YamlConfigSerializer>;

fn parse_u32_input(input: &str, field_name: &str, shared_state: &SharedState) -> Option<u32> {
    match input.parse::<u32>() {
        Ok(value) => Some(value),
        Err(_) => {
            shared_state.set_error(format!("{} must be a number", field_name));
            None
        }
    }
}

pub struct MenuApp {
    client_id: String,
    shared_state: SharedState,
    menu_command_tx: mpsc::UnboundedSender<ClientCommand>,
    create_lobby_dialog: bool,
    lobby_name_input: String,
    max_players_input: String,
    field_width_input: String,
    field_height_input: String,
    tick_interval_input: String,
    wall_collision_mode: WallCollisionMode,
    disconnect_timeout: std::time::Duration,
    disconnecting: Option<std::time::Instant>,
    game_ui: Option<GameUi>,
    window_resized_for_game: bool,
    config_manager: ClientConfigManager,
    server_address_input: String,
}

impl MenuApp {
    pub fn new(
        client_id: String,
        shared_state: SharedState,
        menu_command_tx: mpsc::UnboundedSender<ClientCommand>,
        disconnect_timeout: std::time::Duration,
        config_manager: ClientConfigManager
    ) -> Self {
        let config = config_manager.get_config().unwrap();

        Self {
            client_id,
            shared_state,
            menu_command_tx,
            create_lobby_dialog: false,
            lobby_name_input: String::new(),
            max_players_input: config.lobby.max_players.to_string(),
            field_width_input: config.lobby.field_width.to_string(),
            field_height_input: config.lobby.field_height.to_string(),
            tick_interval_input: config.lobby.tick_interval_ms.to_string(),
            wall_collision_mode: config.lobby.wall_collision_mode,
            disconnecting: None,
            disconnect_timeout,
            game_ui: None,
            window_resized_for_game: false,
            config_manager,
            server_address_input: String::new(),
        }
    }

    fn render_lobby_list(&mut self, ui: &mut egui::Ui, lobbies: &[LobbyInfo]) {
        ui.heading("Snake Game - Lobby List");
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("🔄 Update").clicked() {
                let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::ListLobbies));
            }

            if ui.button("➕ Create Lobby").clicked() {
                self.create_lobby_dialog = true;
                self.lobby_name_input = self.client_id.clone();
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

                    let button_clicked = ui.scope_builder(
                        egui::UiBuilder::new().max_rect(rect),
                        |ui| {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        let full_message = if lobby.current_players == lobby.max_players {
                                            " (Full)"
                                        } else {
                                            ""
                                        };
                                        ui.label(format!("📋 {}", lobby.lobby_name));
                                        ui.label(format!(
                                            "👥 Players: {}/{} {}",
                                            lobby.current_players, lobby.max_players, full_message
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
                        let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::JoinLobby {
                            lobby_id: lobby.lobby_id.clone(),
                        }));
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

                ui.label("Field Width:");
                ui.text_edit_singleline(&mut self.field_width_input);

                ui.label("Field Height:");
                ui.text_edit_singleline(&mut self.field_height_input);

                ui.label("Tick Interval (ms):");
                ui.text_edit_singleline(&mut self.tick_interval_input);

                ui.label("Wall Collision Mode:");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.wall_collision_mode, WallCollisionMode::WrapAround, "Wrap Around");
                    ui.radio_value(&mut self.wall_collision_mode, WallCollisionMode::Death, "Death");
                });

                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        let Some(field_width) = parse_u32_input(&self.field_width_input, "Field width", &self.shared_state) else {
                            return;
                        };

                        let Some(field_height) = parse_u32_input(&self.field_height_input, "Field height", &self.shared_state) else {
                            return;
                        };

                        let Some(max_players) = parse_u32_input(&self.max_players_input, "Max players", &self.shared_state) else {
                            return;
                        };

                        let Some(tick_interval_ms) = parse_u32_input(&self.tick_interval_input, "Tick interval", &self.shared_state) else {
                            return;
                        };

                        let lobby_config = LobbyConfig {
                            max_players,
                            field_width,
                            field_height,
                            wall_collision_mode: self.wall_collision_mode,
                            tick_interval_ms,
                        };

                        let mut config = self.config_manager.get_config().unwrap();
                        config.lobby = lobby_config.clone();

                        // This also validates the config
                        match self.config_manager.set_config(&config) {
                            Ok(_) => {
                                let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::CreateLobby {
                                    name: self.lobby_name_input.clone(),
                                    config: lobby_config
                                }));
                                close_dialog = true;
                            }
                            Err(error) => {
                                self.shared_state.set_error(error);
                                return;
                            }
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
                let is_host = player.client_id == details.creator_id;

                let player_display = match (is_self, is_host) {
                    (true, true) => format!("👤 {} (You, Host)", player.client_id),
                    (true, false) => format!("👤 {} (You)", player.client_id),
                    (false, true) => format!("👤 {} (Host)", player.client_id),
                    (false, false) => format!("👤 {}", player.client_id),
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

        let is_host = self.client_id == details.creator_id;
        let all_ready = details.players.iter().all(|p| p.ready);

        ui.horizontal(|ui| {
            let button_text = if current_ready { "Mark Not Ready" } else { "Mark Ready" };
            if ui.button(button_text).clicked() {
                let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::MarkReady {
                    ready: !current_ready,
                }));
            }

            if ui.button("🚪 Leave Lobby").clicked() {
                let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::LeaveLobby));
            }

            if is_host && all_ready {
                if ui.button("▶ Start Game").clicked() {
                    let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::StartGame));
                }
            }
        });

        ui.separator();
        ui.heading("Events:");

        egui::ScrollArea::vertical()
            .id_salt("events_scroll")
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for event in event_log {
                    ui.label(event);
                }
            });
    }
}

impl eframe::App for MenuApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            if let Some(disconnect_time) = self.disconnecting {
                if disconnect_time.elapsed() < self.disconnect_timeout {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::Disconnect));
                self.disconnecting = Some(std::time::Instant::now());
            }
        }

        if let Some(disconnect_time) = self.disconnecting {
            if disconnect_time.elapsed() >= self.disconnect_timeout {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        if let Some(error) = self.shared_state.get_error() {
            if self.shared_state.get_connection_failed() {
                egui::Window::new("Connection Failed")
                    .collapsible(false)
                    .show(ctx, |ui| {
                        ui.label(&error);
                        ui.add_space(10.0);
                        ui.label("Enter server address:");
                        ui.text_edit_singleline(&mut self.server_address_input);
                        ui.add_space(5.0);

                        ui.horizontal(|ui| {
                            if ui.button("Retry").clicked() {
                                let address = if self.server_address_input.trim().is_empty() {
                                    "http://localhost:5001".to_string()
                                } else {
                                    self.server_address_input.clone()
                                };

                                self.shared_state.set_retry_server_address(Some(address));
                                self.shared_state.clear_error();
                                self.shared_state.set_connection_failed(false);
                            }

                            if ui.button("Quit").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                    });
            } else {
                egui::Window::new("Error")
                    .collapsible(false)
                    .show(ctx, |ui| {
                        ui.label(&error);
                        if ui.button("OK").clicked() {
                            self.shared_state.clear_error();
                            if self.shared_state.should_close() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                    });
            }
        }

        if self.create_lobby_dialog {
            self.render_create_lobby_dialog(ctx);
        }

        let current_state = self.shared_state.get_state();

        if let AppState::InGame { game_state: Some(ref state), .. } = current_state {
            if !self.window_resized_for_game {
                let pixels_per_cell = 64.0;
                let game_width = state.field_width as f32 * pixels_per_cell;
                let game_height = state.field_height as f32 * pixels_per_cell;

                let padding = 100.0;
                let window_width = game_width + padding;
                let window_height = game_height + padding + 100.0;

                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(window_width, window_height)));
                self.window_resized_for_game = true;
            }
        } else {
            self.window_resized_for_game = false;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            match current_state {
                AppState::LobbyList { lobbies } => {
                    self.render_lobby_list(ui, &lobbies);
                }
                AppState::InLobby { details, event_log } => {
                    self.render_in_lobby(ui, &details, &event_log);
                }
                AppState::InGame { session_id, game_state } => {
                    if self.game_ui.is_none() {
                        self.game_ui = Some(GameUi::new());
                    }
                    if let Some(game_ui) = &mut self.game_ui {
                        game_ui.render_game(ui, ctx, &session_id, &game_state, &self.client_id, &self.menu_command_tx);
                    }
                }
                AppState::GameOver { scores, winner_id, last_game_state, reason, play_again_status } => {
                    if self.game_ui.is_none() {
                        self.game_ui = Some(GameUi::new());
                    }
                    if let Some(game_ui) = &mut self.game_ui {
                        game_ui.render_game_over(ui, ctx, &scores, &winner_id, &self.client_id, &last_game_state, &reason, &play_again_status, &self.menu_command_tx);
                    }
                }
            }
        });

        if self.disconnecting.is_some() {
            ctx.request_repaint();
        }
    }
}
