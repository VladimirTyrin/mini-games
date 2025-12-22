use eframe::egui;
use common::{LobbyInfo, LobbyDetails};
use tokio::sync::mpsc;
use crate::state::{MenuCommand, SharedState, AppState};
use crate::game_ui::GameUi;

pub struct MenuApp {
    client_id: String,
    shared_state: SharedState,
    menu_command_tx: mpsc::UnboundedSender<MenuCommand>,
    create_lobby_dialog: bool,
    lobby_name_input: String,
    max_players_input: String,
    disconnect_timeout: std::time::Duration,
    disconnecting: Option<std::time::Instant>,
    game_ui: Option<GameUi>,
    window_resized_for_game: bool,
}

impl MenuApp {
    pub fn new(
        client_id: String,
        shared_state: SharedState,
        menu_command_tx: mpsc::UnboundedSender<MenuCommand>,
        disconnect_timeout: std::time::Duration
    ) -> Self {
        Self {
            client_id,
            shared_state,
            menu_command_tx,
            create_lobby_dialog: false,
            lobby_name_input: String::new(),
            max_players_input: "4".to_string(),
            disconnecting: None,
            disconnect_timeout,
            game_ui: None,
            window_resized_for_game: false,
        }
    }

    fn render_lobby_list(&mut self, ui: &mut egui::Ui, lobbies: &[LobbyInfo]) {
        ui.heading("Snake Game - Lobby List");
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("🔄 Update").clicked() {
                let _ = self.menu_command_tx.send(MenuCommand::ListLobbies);
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
                        let _ = self.menu_command_tx.send(MenuCommand::JoinLobby {
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
                                let _ = self.menu_command_tx.send(MenuCommand::CreateLobby {
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
                let _ = self.menu_command_tx.send(MenuCommand::MarkReady {
                    ready: !current_ready,
                });
            }

            if ui.button("🚪 Leave Lobby").clicked() {
                let _ = self.menu_command_tx.send(MenuCommand::LeaveLobby);
            }

            if is_host && all_ready {
                if ui.button("▶ Start Game").clicked() {
                    let _ = self.menu_command_tx.send(MenuCommand::StartGame);
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
                let _ = self.menu_command_tx.send(MenuCommand::Disconnect);
                self.disconnecting = Some(std::time::Instant::now());
            }
        }

        if let Some(disconnect_time) = self.disconnecting {
            if disconnect_time.elapsed() >= self.disconnect_timeout {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        if let Some(error) = self.shared_state.get_error() {
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
                        self.game_ui = Some(GameUi::new(self.shared_state.clone()));
                    }
                    if let Some(game_ui) = &mut self.game_ui {
                        game_ui.render_game(ui, ctx, &session_id, &game_state, &self.client_id);
                    }
                }
                AppState::GameOver { scores, winner_id, last_game_state } => {
                    if self.game_ui.is_none() {
                        self.game_ui = Some(GameUi::new(self.shared_state.clone()));
                    }
                    if let Some(game_ui) = &mut self.game_ui {
                        game_ui.render_game_over(ui, ctx, &scores, &winner_id, &self.client_id, &last_game_state, &self.menu_command_tx);
                    }
                }
            }
        });

        if self.disconnecting.is_some() {
            ctx.request_repaint();
        }
    }
}
