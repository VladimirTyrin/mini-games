use crate::config::{Config, LobbyConfig};
use crate::game_ui::GameUi;
use crate::sprites::Sprites;
use crate::state::{AppState, MenuCommand, ClientCommand, SharedState};
use crate::colors::generate_color_from_client_id;
use common::config::{ConfigManager, FileContentConfigProvider, YamlConfigSerializer};
use common::{Direction, WallCollisionMode};
use common::{LobbyDetails, LobbyInfo};
use eframe::egui;
use egui::{Align, Layout};
use ringbuffer::{AllocRingBuffer, RingBuffer};
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

fn parse_f32_input(input: &str, field_name: &str, shared_state: &SharedState) -> Option<f32> {
    match input.parse::<f32>() {
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
    max_food_count_input: String,
    food_spawn_probability_input: String,
    wall_collision_mode: WallCollisionMode,
    dead_snake_behavior: common::DeadSnakeBehavior,
    selected_bot_type: common::BotType,
    disconnect_timeout: std::time::Duration,
    disconnecting: Option<std::time::Instant>,
    game_ui: Option<GameUi>,
    window_resized_for_game: bool,
    config_manager: ClientConfigManager,
    server_address_input: String,
    sprites: Sprites,
    chat_input: String,
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
            max_food_count_input: config.lobby.max_food_count.to_string(),
            food_spawn_probability_input: config.lobby.food_spawn_probability.to_string(),
            wall_collision_mode: config.lobby.wall_collision_mode,
            dead_snake_behavior: config.lobby.dead_snake_behavior,
            selected_bot_type: common::BotType::Efficient,
            disconnecting: None,
            disconnect_timeout,
            game_ui: None,
            window_resized_for_game: false,
            config_manager,
            server_address_input: String::new(),
            sprites: Sprites::load(),
            chat_input: String::new(),
        }
    }

    fn render_chat_widget(&mut self, ui: &mut egui::Ui, chat_messages: &AllocRingBuffer<String>, is_lobby_list: bool) {
        ui.separator();
        ui.heading("Chat");

        let available_height = ui.available_height();
        let input_height = 30.0;
        let messages_height = available_height - input_height - 10.0;

        egui::ScrollArea::vertical()
            .id_salt(if is_lobby_list { "lobby_list_chat_scroll" } else { "in_lobby_chat_scroll" })
            .stick_to_bottom(true)
            .max_height(messages_height)
            .show(ui, |ui| {
                if chat_messages.is_empty() {
                    ui.label(egui::RichText::new("No messages yet...").italics().color(egui::Color32::GRAY));
                } else {
                    for message in chat_messages {
                        ui.label(message);
                    }
                }
            });

        ui.add_space(5.0);

        let response = ui.add_sized(
            egui::vec2(ui.available_width(), input_height),
            egui::TextEdit::singleline(&mut self.chat_input)
                .hint_text("Type message and press Enter...")
        );

        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let message = self.chat_input.trim();
            if !message.is_empty() {
                let command = if is_lobby_list {
                    ClientCommand::Menu(MenuCommand::LobbyListChatMessage {
                        message: message.to_string(),
                    })
                } else {
                    ClientCommand::Menu(MenuCommand::InLobbyChatMessage {
                        message: message.to_string(),
                    })
                };
                let _ = self.menu_command_tx.send(command);
                self.chat_input.clear();
                response.request_focus();
            }
        }
    }

    fn render_lobby_list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, lobbies: &[LobbyInfo], chat_messages: &AllocRingBuffer<String>) {
        let available_height = ui.available_height();
        let chat_height = 200.0;
        let main_content_height = available_height - chat_height;

        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), main_content_height),
            Layout::top_down(Align::LEFT),
            |ui| {
                ui.heading("Snake Game - Lobby List");
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("🔄 Update (F5)").clicked() {
                        let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::ListLobbies));
                    }

                    if ui.button("➕ Create Lobby (Ctrl+N)").clicked() {
                        self.create_lobby_dialog = true;
                        self.lobby_name_input = self.client_id.clone();
                        let config = self.config_manager.get_config().unwrap();
                        self.max_players_input = config.lobby.max_players.to_string();
                    }
                });

                ctx.input(|i| {
                    if i.key_pressed(egui::Key::F5) {
                        let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::ListLobbies));
                    }
                    if i.modifiers.ctrl && i.key_pressed(egui::Key::N) {
                        self.create_lobby_dialog = true;
                        self.lobby_name_input = self.client_id.clone();
                        let config = self.config_manager.get_config().unwrap();
                        self.max_players_input = config.lobby.max_players.to_string();
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

                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
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
        );

        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), chat_height),
            Layout::top_down(Align::LEFT),
            |ui| {
                self.render_chat_widget(ui, chat_messages, true);
            }
        );
    }

    fn render_create_lobby_dialog(&mut self, ctx: &egui::Context) {
        let mut close_dialog = false;
        let mut create_lobby = false;

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

                ui.label("Max Food Count:");
                ui.text_edit_singleline(&mut self.max_food_count_input);

                ui.label("Food Spawn Probability:");
                ui.text_edit_singleline(&mut self.food_spawn_probability_input);

                ui.label("Wall Collision Mode:");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.wall_collision_mode, WallCollisionMode::WrapAround, "Wrap Around");
                    ui.radio_value(&mut self.wall_collision_mode, WallCollisionMode::Death, "Death");
                });

                ui.label("Dead Snake Behavior:");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.dead_snake_behavior, common::DeadSnakeBehavior::Disappear, "Disappear");
                    ui.radio_value(&mut self.dead_snake_behavior, common::DeadSnakeBehavior::StayOnField, "Stay On Field");
                });

                ui.horizontal(|ui| {
                    if ui.button("Create (Enter)").clicked() {
                        create_lobby = true;
                    }

                    if ui.button("Cancel (Esc)").clicked() {
                        close_dialog = true;
                    }
                });

                ctx.input(|i| {
                    if i.key_pressed(egui::Key::Enter) {
                        create_lobby = true;
                    }
                    if i.key_pressed(egui::Key::Escape) {
                        close_dialog = true;
                    }
                });
            });

        if create_lobby {
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

            let Some(max_food_count) = parse_u32_input(&self.max_food_count_input, "Max food count", &self.shared_state) else {
                return;
            };

            let Some(food_spawn_probability) = parse_f32_input(&self.food_spawn_probability_input, "Food spawn probability", &self.shared_state) else {
                return;
            };

            let lobby_config = LobbyConfig {
                max_players,
                field_width,
                field_height,
                wall_collision_mode: self.wall_collision_mode,
                dead_snake_behavior: self.dead_snake_behavior,
                tick_interval_ms,
                max_food_count,
                food_spawn_probability,
            };

            let mut config = self.config_manager.get_config().unwrap();
            config.lobby = lobby_config.clone();

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

        if close_dialog {
            self.create_lobby_dialog = false;
        }
    }

    fn render_in_lobby(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, details: &LobbyDetails, event_log: &AllocRingBuffer<String>) {
        let available_height = ui.available_height();
        let chat_height = 200.0;
        let main_content_height = available_height - chat_height;

        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), main_content_height),
            Layout::top_down(Align::LEFT),
            |ui| {
                ui.heading(format!("Lobby: {}", details.lobby_name));
                ui.separator();

                ui.label(format!("Lobby ID: {}", details.lobby_id));
                ui.label(format!("Players: {}/{}", details.players.len(), details.max_players));

                ui.separator();
                ui.heading("Players:");

                let creator_id = details.creator.as_ref()
                    .map(|c| c.player_id.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                let is_host = self.client_id == creator_id;

                for player in &details.players {
                    ui.horizontal(|ui| {
                        let player_id = player.identity.as_ref()
                            .map(|i| i.player_id.clone())
                            .unwrap_or_else(|| "Unknown".to_string());

                        let is_bot = player.identity.as_ref().map(|i| i.is_bot).unwrap_or(false);
                        let bot_type_suffix = if is_bot {
                            let bot_type = player.identity.as_ref()
                                .and_then(|i| common::BotType::try_from(i.bot_type).ok())
                                .unwrap_or(common::BotType::Unspecified);
                            match bot_type {
                                common::BotType::Efficient => " (Bot - Efficient)",
                                common::BotType::Random => " (Bot - Random)",
                                _ => " (Bot)",
                            }
                        } else {
                            ""
                        };

                        let is_self = !is_bot && player_id == self.client_id;
                        let is_player_host = player_id == creator_id;

                        let player_display = match (is_self, is_player_host) {
                            (true, true) => format!("👤 {} (You, Host)", player_id),
                            (true, false) => format!("👤 {} (You)", player_id),
                            (false, true) => format!("👤 {} (Host){}", player_id, bot_type_suffix),
                            (false, false) => format!("👤 {}{}", player_id, bot_type_suffix),
                        };

                        let color = generate_color_from_client_id(&player_id);
                        let head_sprite = self.sprites.get_head_sprite(Direction::Right);
                        let texture = head_sprite.to_egui_texture(ctx, &format!("lobby_head_{}", player_id));

                        let icon_size = 20.0;
                        let (rect, _response) = ui.allocate_exact_size(
                            egui::vec2(icon_size, icon_size),
                            egui::Sense::hover()
                        );

                        ui.painter().image(
                            texture.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            color,
                        );

                        ui.label(player_display);

                        if player.ready {
                            ui.label("✅ Ready");
                        } else {
                            ui.label("⏳ Not Ready");
                        }

                        if is_host && !is_self {
                            if ui.button("❌ Kick").clicked() {
                                let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::KickFromLobby {
                                    player_id: player_id.clone(),
                                }));
                            }
                        }
                    });
                }

                ui.separator();

                let current_ready = details.players
                    .iter()
                    .find(|p| {
                        p.identity.as_ref()
                            .map(|i| !i.is_bot && i.player_id == self.client_id)
                            .unwrap_or(false)
                    })
                    .map(|p| p.ready)
                    .unwrap_or(false);

                let all_ready = details.players.iter().all(|p| p.ready);

                ui.horizontal(|ui| {
                    let button_text = if current_ready { "Mark Not Ready (Ctrl+R)" } else { "Mark Ready (Ctrl+R)" };
                    if ui.button(button_text).clicked() {
                        let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::MarkReady {
                            ready: !current_ready,
                        }));
                    }

                    if ui.button("🚪 Leave Lobby (Esc)").clicked() {
                        let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::LeaveLobby));
                    }

                    if is_host && all_ready {
                        if ui.button("▶ Start Game (Ctrl+S)").clicked() {
                            let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::StartGame));
                        }
                    }
                });

                ctx.input(|i| {
                    if i.key_pressed(egui::Key::Escape) {
                        let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::LeaveLobby));
                    }
                    if i.modifiers.ctrl && i.key_pressed(egui::Key::R) {
                        let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::MarkReady {
                            ready: !current_ready,
                        }));
                    }
                    if is_host && all_ready && i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                        let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::StartGame));
                    }
                });

                if is_host {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_label("Bot Type")
                            .selected_text(match self.selected_bot_type {
                                common::BotType::Efficient => "Efficient",
                                common::BotType::Random => "Random",
                                _ => "Unknown",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.selected_bot_type, common::BotType::Efficient, "Efficient");
                                ui.selectable_value(&mut self.selected_bot_type, common::BotType::Random, "Random");
                            });

                        if ui.button("🤖 Add Bot (Ctrl+B)").clicked() {
                            let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::AddBot {
                                bot_type: self.selected_bot_type,
                            }));
                        }
                    });

                    ctx.input(|i| {
                        if i.modifiers.ctrl && i.key_pressed(egui::Key::B) {
                            let _ = self.menu_command_tx.send(ClientCommand::Menu(MenuCommand::AddBot {
                                bot_type: self.selected_bot_type,
                            }));
                        }
                    });
                }
            }
        );

        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), chat_height),
            Layout::top_down(Align::LEFT),
            |ui| {
                self.render_chat_widget(ui, event_log, false);
            }
        );
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

        let title = if let Some(ping) = self.shared_state.get_ping() {
            format!("Snake Game - Ping: {}ms", ping)
        } else {
            "Snake Game".to_string()
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

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
                AppState::LobbyList { lobbies, chat_messages } => {
                    self.render_lobby_list(ui, ctx, &lobbies, &chat_messages);
                }
                AppState::InLobby { details, event_log } => {
                    self.render_in_lobby(ui, ctx, &details, &event_log);
                }
                AppState::InGame { session_id, game_state } => {
                    if self.game_ui.is_none() {
                        self.game_ui = Some(GameUi::new());
                    }
                    if let Some(game_ui) = &mut self.game_ui {
                        game_ui.render_game(ui, ctx, &session_id, &game_state, &self.client_id, &self.menu_command_tx);
                    }
                }
                AppState::GameOver { scores, winner, last_game_state, reason, play_again_status } => {
                    if self.game_ui.is_none() {
                        self.game_ui = Some(GameUi::new());
                    }
                    if let Some(game_ui) = &mut self.game_ui {
                        game_ui.render_game_over(ui, ctx, &scores, &winner, &self.client_id, &last_game_state, &reason, &play_again_status, &self.menu_command_tx);
                    }
                }
            }
        });

        if self.disconnecting.is_some() {
            ctx.request_repaint();
        }
    }
}
