use crate::config::{Config, GameType, SnakeLobbyConfig};
use crate::game_ui::GameUi;
use crate::sprites::Sprites;
use crate::state::{AppState, MenuCommand, ClientCommand, SharedState, LobbyConfig};
use crate::colors::generate_color_from_client_id;
use crate::CommandSender;
use common::config::{ConfigManager, FileContentConfigProvider, YamlConfigSerializer};
use common::{proto::snake::{Direction, SnakeBotType}, WallCollisionMode};
use common::{LobbyDetails, LobbyInfo};
use eframe::egui;
use egui::{Align, Layout};
use ringbuffer::{AllocRingBuffer, RingBuffer};

type ClientConfigManager = ConfigManager<FileContentConfigProvider, Config, YamlConfigSerializer>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppStateType {
    LobbyList,
    InLobby,
    InGame,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatLocation {
    LobbyList,
    InLobby,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatHeading {
    Show,
    Hide,
}

impl AppStateType {
    fn from(state: &AppState) -> Self {
        match state {
            AppState::LobbyList { .. } => Self::LobbyList,
            AppState::InLobby { .. } => Self::InLobby,
            AppState::InGame { .. } => Self::InGame,
            AppState::GameOver { .. } => Self::GameOver,
        }
    }
}

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
    command_sender: CommandSender,
    offline_command_sender: Option<CommandSender>,
    create_lobby_dialog: bool,
    creating_offline_game: bool,
    selected_game_type: GameType,
    lobby_name_input: String,
    max_players_input: String,
    field_width_input: String,
    field_height_input: String,
    tick_interval_input: String,
    max_food_count_input: String,
    food_spawn_probability_input: String,
    wall_collision_mode: WallCollisionMode,
    dead_snake_behavior: common::DeadSnakeBehavior,
    selected_snake_bot_type: SnakeBotType,
    selected_ttt_bot_type: common::TicTacToeBotType,
    win_count_input: String,
    disconnect_timeout: std::time::Duration,
    disconnecting: Option<std::time::Instant>,
    game_ui: Option<GameUi>,
    config_manager: ClientConfigManager,
    server_address_input: String,
    sprites: Sprites,
    chat_input: String,
    normal_window_size: Option<egui::Vec2>,
    previous_app_state: Option<AppStateType>,
}

impl MenuApp {
    pub fn new(
        client_id: String,
        shared_state: SharedState,
        command_sender: CommandSender,
        disconnect_timeout: std::time::Duration,
        config_manager: ClientConfigManager
    ) -> Self {
        let config = config_manager.get_config().unwrap();

        Self {
            client_id,
            shared_state,
            command_sender,
            offline_command_sender: None,
            create_lobby_dialog: false,
            creating_offline_game: false,
            selected_game_type: config.last_game.unwrap_or(GameType::Snake),
            lobby_name_input: String::new(),
            max_players_input: config.snake.max_players.to_string(),
            field_width_input: config.snake.field_width.to_string(),
            field_height_input: config.snake.field_height.to_string(),
            tick_interval_input: config.snake.tick_interval_ms.to_string(),
            max_food_count_input: config.snake.max_food_count.to_string(),
            food_spawn_probability_input: config.snake.food_spawn_probability.to_string(),
            wall_collision_mode: config.snake.wall_collision_mode,
            dead_snake_behavior: config.snake.dead_snake_behavior,
            selected_snake_bot_type: SnakeBotType::Efficient,
            selected_ttt_bot_type: common::TicTacToeBotType::TictactoeBotTypeMinimax,
            win_count_input: config.tictactoe.win_count.to_string(),
            disconnecting: None,
            disconnect_timeout,
            game_ui: None,
            config_manager,
            server_address_input: String::new(),
            sprites: Sprites::load(),
            chat_input: String::new(),
            normal_window_size: None,
            previous_app_state: None,
        }
    }

    fn handle_state_transition(
        &mut self,
        from: &Option<AppStateType>,
        to: &AppStateType,
        ctx: &egui::Context,
    ) {
        match (from, to) {
            (_, AppStateType::InGame) => {
                if self.normal_window_size.is_none() {
                    self.normal_window_size = Some(
                        ctx.input(|i| i.viewport().inner_rect)
                            .map(|r| r.size())
                            .unwrap_or(egui::vec2(600.0, 700.0))
                    );
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }

            (Some(AppStateType::GameOver), AppStateType::LobbyList) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                if let Some(size) = self.normal_window_size {
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
                }
            }

            _ => {}
        }
    }

    fn render_chat_widget(&mut self, ui: &mut egui::Ui, chat_messages: &AllocRingBuffer<String>, location: ChatLocation, heading: ChatHeading) {
        if heading == ChatHeading::Show {
            ui.separator();
            ui.heading("Chat");
        }

        let input_height = 30.0;
        let available_width = ui.available_width();

        let scroll_id = match location {
            ChatLocation::LobbyList => "lobby_list_chat_scroll",
            ChatLocation::InLobby => "in_lobby_chat_scroll",
        };

        let mut response_opt: Option<egui::Response> = None;

        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            response_opt = Some(ui.add_sized(
                egui::vec2(available_width, input_height),
                egui::TextEdit::singleline(&mut self.chat_input)
                    .hint_text("Type message and press Enter...")
            ));

            ui.add_space(5.0);

            ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(scroll_id)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.set_min_width(available_width - 15.0);
                        if chat_messages.is_empty() {
                            ui.label(egui::RichText::new("No messages yet...").italics().color(egui::Color32::GRAY));
                        } else {
                            for message in chat_messages {
                                ui.label(message);
                            }
                        }
                    });
            });
        });

        if let Some(response) = response_opt {
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let message = self.chat_input.trim();
                if !message.is_empty() {
                    let command = match location {
                        ChatLocation::LobbyList => ClientCommand::Menu(MenuCommand::LobbyListChatMessage {
                            message: message.to_string(),
                        }),
                        ChatLocation::InLobby => ClientCommand::Menu(MenuCommand::InLobbyChatMessage {
                            message: message.to_string(),
                        }),
                    };
                    self.command_sender.send(command);
                    self.chat_input.clear();
                    response.request_focus();
                }
            }
        }
    }

    fn render_lobby_list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, lobbies: &[LobbyInfo], chat_messages: &AllocRingBuffer<String>) {
        let is_offline = self.shared_state.get_connection_mode() == crate::state::ConnectionMode::TemporaryOffline;

        if is_offline {
            self.render_offline_lobby_list(ui, ctx);
        } else {
            self.render_online_lobby_list(ui, ctx, lobbies, chat_messages);
        }
    }

    fn render_offline_lobby_list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Snake Game - Offline Mode");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Not connected to server.");
            if ui.button("🔌 Connect (F5)").clicked() {
                self.shared_state.set_connection_mode(crate::state::ConnectionMode::Online);
                self.shared_state.set_connection_failed(true);
                self.shared_state.set_error("Enter server address to connect".to_string());
            }
        });

        ctx.input(|i| {
            if i.key_pressed(egui::Key::F5) {
                self.shared_state.set_connection_mode(crate::state::ConnectionMode::Online);
                self.shared_state.set_connection_failed(true);
                self.shared_state.set_error("Enter server address to connect".to_string());
            }
        });

        ui.separator();
        ui.add_space(20.0);

        ui.vertical_centered(|ui| {
            ui.label("Play offline against bots:");
            ui.add_space(10.0);

            if ui.button("🎮 Create Offline Game (Ctrl+N)").clicked() {
                self.create_lobby_dialog = true;
                self.creating_offline_game = true;
                self.lobby_name_input = "Offline Game".to_string();
                let config = self.config_manager.get_config().unwrap();
                self.max_players_input = config.snake.max_players.to_string();
            }
        });

        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::N) {
                self.create_lobby_dialog = true;
                self.creating_offline_game = true;
                self.lobby_name_input = "Offline Game".to_string();
                let config = self.config_manager.get_config().unwrap();
                self.max_players_input = config.snake.max_players.to_string();
            }
        });
    }

    fn render_online_lobby_list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, lobbies: &[LobbyInfo], chat_messages: &AllocRingBuffer<String>) {
        let main_content_height = 250.0;

        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), main_content_height),
            Layout::top_down(Align::LEFT),
            |ui| {
                ui.heading("Snake Game - Lobby List");
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("🔄 Update (F5)").clicked() {
                        self.command_sender.send(ClientCommand::Menu(MenuCommand::ListLobbies));
                    }

                    if ui.button("➕ Create Lobby (Ctrl+N)").clicked() {
                        self.create_lobby_dialog = true;
                        self.creating_offline_game = false;
                        self.lobby_name_input = self.client_id.clone();
                        let config = self.config_manager.get_config().unwrap();
                        self.max_players_input = config.snake.max_players.to_string();
                    }

                    if ui.button("🎮 Offline Game (Ctrl+O)").clicked() {
                        self.create_lobby_dialog = true;
                        self.creating_offline_game = true;
                        self.lobby_name_input = "Offline Game".to_string();
                        let config = self.config_manager.get_config().unwrap();
                        self.max_players_input = config.snake.max_players.to_string();
                    }
                });

                ctx.input(|i| {
                    if i.key_pressed(egui::Key::F5) {
                        self.command_sender.send(ClientCommand::Menu(MenuCommand::ListLobbies));
                    }
                    if i.modifiers.ctrl && i.key_pressed(egui::Key::N) {
                        self.create_lobby_dialog = true;
                        self.creating_offline_game = false;
                        self.lobby_name_input = self.client_id.clone();
                        let config = self.config_manager.get_config().unwrap();
                        self.max_players_input = config.snake.max_players.to_string();
                    }
                    if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                        self.create_lobby_dialog = true;
                        self.creating_offline_game = true;
                        self.lobby_name_input = "Offline Game".to_string();
                        let config = self.config_manager.get_config().unwrap();
                        self.max_players_input = config.snake.max_players.to_string();
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
                                self.command_sender.send(ClientCommand::Menu(MenuCommand::JoinLobby {
                                    lobby_id: lobby.lobby_id.clone(),
                                    join_as_observer: false,
                                }));
                            }
                        }
                    });
                }
            }
        );

        self.render_chat_widget(ui, chat_messages, ChatLocation::LobbyList, ChatHeading::Show);
    }

    fn render_create_lobby_dialog(&mut self, ctx: &egui::Context) {
        let mut close_dialog = false;
        let mut create_lobby = false;

        let title = if self.creating_offline_game { "Create Offline Game" } else { "Create Lobby" };
        egui::Window::new(title)
            .open(&mut self.create_lobby_dialog)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label("Game Type:");
                ui.horizontal(|ui| {
                    if ui.radio_value(&mut self.selected_game_type, GameType::Snake, "Snake").clicked() {
                        let config = self.config_manager.get_config().unwrap();
                        self.max_players_input = config.snake.max_players.to_string();
                        self.field_width_input = config.snake.field_width.to_string();
                        self.field_height_input = config.snake.field_height.to_string();
                    }
                    if ui.radio_value(&mut self.selected_game_type, GameType::TicTacToe, "TicTacToe").clicked() {
                        let config = self.config_manager.get_config().unwrap();
                        self.max_players_input = "2".to_string();
                        self.field_width_input = config.tictactoe.field_width.to_string();
                        self.field_height_input = config.tictactoe.field_height.to_string();
                        self.win_count_input = config.tictactoe.win_count.to_string();
                    }
                });

                ui.separator();

                ui.label("Lobby Name:");
                ui.text_edit_singleline(&mut self.lobby_name_input);

                ui.label("Max Players:");
                let max_players_enabled = self.selected_game_type == GameType::Snake;
                ui.add_enabled(max_players_enabled, egui::TextEdit::singleline(&mut self.max_players_input));
                if self.selected_game_type == GameType::TicTacToe {
                    ui.label("  (Fixed at 2 for TicTacToe)");
                }

                ui.label("Field Width:");
                ui.text_edit_singleline(&mut self.field_width_input);

                ui.label("Field Height:");
                ui.text_edit_singleline(&mut self.field_height_input);

                match self.selected_game_type {
                    GameType::Snake => {
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
                    }
                    GameType::TicTacToe => {
                        ui.label("Win Count:");
                        ui.text_edit_singleline(&mut self.win_count_input);
                    }
                }

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

            let lobby_config = match self.selected_game_type {
                GameType::Snake => {
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

                    let snake_config = SnakeLobbyConfig {
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
                    config.snake = snake_config.clone();
                    config.last_game = Some(GameType::Snake);
                    self.config_manager.set_config(&config).ok();

                    LobbyConfig::Snake(snake_config)
                }
                GameType::TicTacToe => {
                    let Some(win_count) = parse_u32_input(&self.win_count_input, "Win count", &self.shared_state) else {
                        return;
                    };

                    let ttt_config = crate::config::TicTacToeLobbyConfig {
                        field_width,
                        field_height,
                        win_count,
                    };

                    let mut config = self.config_manager.get_config().unwrap();
                    config.tictactoe = ttt_config.clone();
                    config.last_game = Some(GameType::TicTacToe);
                    self.config_manager.set_config(&config).ok();

                    LobbyConfig::TicTacToe(ttt_config)
                }
            };

            if self.creating_offline_game {
                let sender = self.get_or_create_offline_sender();
                sender.send(ClientCommand::Menu(MenuCommand::CreateLobby {
                    name: self.lobby_name_input.clone(),
                    config: lobby_config
                }));
            } else {
                self.command_sender.send(ClientCommand::Menu(MenuCommand::CreateLobby {
                    name: self.lobby_name_input.clone(),
                    config: lobby_config
                }));
            }
            close_dialog = true;
        }

        if close_dialog {
            self.create_lobby_dialog = false;
        }
    }

    fn get_or_create_offline_sender(&mut self) -> CommandSender {
        if let Some(ref sender) = self.offline_command_sender {
            return sender.clone();
        }

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let sender = CommandSender::Local(tx);
        self.offline_command_sender = Some(sender.clone());

        let client_id = self.client_id.clone();
        let shared_state = self.shared_state.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                crate::offline::local_game_task(client_id, shared_state, rx).await;
            });
        });

        sender
    }

    fn render_in_lobby(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, details: &LobbyDetails, event_log: &AllocRingBuffer<String>) {
        let is_offline_lobby = details.lobby_id == "offline";
        let cmd_sender = if is_offline_lobby {
            self.offline_command_sender.clone().unwrap_or_else(|| self.command_sender.clone())
        } else {
            self.command_sender.clone()
        };

        let creator_id = details.creator.as_ref()
            .map(|c| c.player_id.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        let is_host = self.client_id == creator_id;

        let is_snake_lobby = details.settings.as_ref()
            .map(|s| matches!(s, common::lobby_details::Settings::Snake(_)))
            .unwrap_or(false);

        let is_tictactoe_lobby = details.settings.as_ref()
            .map(|s| matches!(s, common::lobby_details::Settings::Tictactoe(_)))
            .unwrap_or(false);

        let has_enough_players = if is_tictactoe_lobby {
            details.players.len() == 2
        } else {
            details.players.len() >= 1
        };

        let is_self_observer = details.observers.iter()
            .any(|o| o.player_id == self.client_id);
        let lobby_is_full = details.players.len() >= details.max_players as usize;

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
        let can_start = is_host && all_ready && has_enough_players;

        ui.heading(format!("Lobby: {}", details.lobby_name));
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(format!("Lobby ID: {}", details.lobby_id));
            ui.separator();
            ui.label(format!("Players: {}/{}", details.players.len(), details.max_players));
        });
        ui.separator();

        let panels_height = 200.0;

        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), panels_height),
            Layout::left_to_right(Align::TOP),
            |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(220.0, panels_height),
                    Layout::top_down(Align::LEFT),
                    |ui| {
                        ui.heading("Commands");
                        ui.add_space(5.0);

                        if is_self_observer {
                            if !lobby_is_full {
                                if ui.button("👤 Become Player (Ctrl+P)").clicked() {
                                    cmd_sender.send(ClientCommand::Menu(MenuCommand::BecomePlayer));
                                }
                            } else {
                                ui.add_enabled(false, egui::Button::new("👤 Become Player (Lobby Full)"));
                            }
                        } else {
                            let button_text = if current_ready { "Mark Not Ready (Ctrl+R)" } else { "Mark Ready (Ctrl+R)" };
                            if ui.button(button_text).clicked() {
                                cmd_sender.send(ClientCommand::Menu(MenuCommand::MarkReady {
                                    ready: !current_ready,
                                }));
                            }

                            if ui.button("👁 Become Observer (Ctrl+O)").clicked() {
                                cmd_sender.send(ClientCommand::Menu(MenuCommand::BecomeObserver));
                            }
                        }

                        if ui.button("🚪 Leave Lobby (Esc)").clicked() {
                            cmd_sender.send(ClientCommand::Menu(MenuCommand::LeaveLobby));
                        }

                        if can_start {
                            if ui.button("▶ Start Game (Ctrl+S)").clicked() {
                                cmd_sender.send(ClientCommand::Menu(MenuCommand::StartGame));
                            }
                        } else if is_host {
                            let reason = if !all_ready {
                                "Not all players are ready"
                            } else if !has_enough_players {
                                if is_tictactoe_lobby {
                                    "TicTacToe requires exactly 2 players"
                                } else {
                                    "Need at least 1 player"
                                }
                            } else {
                                ""
                            };
                            ui.add_enabled(false, egui::Button::new(format!("▶ Start Game ({})", reason)));
                        }

                        if is_host {
                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(5.0);

                            if is_snake_lobby {
                                egui::ComboBox::from_label("Bot Type")
                                    .selected_text(match self.selected_snake_bot_type {
                                        SnakeBotType::Efficient => "Efficient",
                                        SnakeBotType::Random => "Random",
                                        _ => "Unknown",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.selected_snake_bot_type, SnakeBotType::Efficient, "Efficient");
                                        ui.selectable_value(&mut self.selected_snake_bot_type, SnakeBotType::Random, "Random");
                                    });

                                if ui.button("+ Add Bot (Ctrl+B)").clicked() {
                                    cmd_sender.send(ClientCommand::Menu(MenuCommand::AddBot {
                                        bot_type: crate::state::BotType::Snake(self.selected_snake_bot_type),
                                    }));
                                }
                            } else if is_tictactoe_lobby {
                                egui::ComboBox::from_label("Bot Type")
                                    .selected_text(match self.selected_ttt_bot_type {
                                        common::TicTacToeBotType::TictactoeBotTypeRandom => "Random",
                                        common::TicTacToeBotType::TictactoeBotTypeMinimax => "Minimax",
                                        _ => "Unknown",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.selected_ttt_bot_type, common::TicTacToeBotType::TictactoeBotTypeRandom, "Random");
                                        ui.selectable_value(&mut self.selected_ttt_bot_type, common::TicTacToeBotType::TictactoeBotTypeMinimax, "Minimax");
                                    });

                                if ui.button("+ Add Bot (Ctrl+B)").clicked() {
                                    cmd_sender.send(ClientCommand::Menu(MenuCommand::AddBot {
                                        bot_type: crate::state::BotType::TicTacToe(self.selected_ttt_bot_type),
                                    }));
                                }
                            }
                        }
                    }
                );

                ui.separator();

                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), panels_height),
                    Layout::top_down(Align::LEFT),
                    |ui| {
                        ui.heading("Players");
                        ui.add_space(5.0);

                        for player in &details.players {
                            ui.horizontal(|ui| {
                                let player_id = player.identity.as_ref()
                                    .map(|i| i.player_id.clone())
                                    .unwrap_or_else(|| "Unknown".to_string());

                                let is_bot = player.identity.as_ref().map(|i| i.is_bot).unwrap_or(false);
                                let bot_type_suffix = if is_bot { " (Bot)" } else { "" };

                                let is_self = !is_bot && player_id == self.client_id;
                                let is_player_host = player_id == creator_id;

                                if is_snake_lobby {
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
                                }

                                let player_display = match (is_self, is_player_host) {
                                    (true, true) => format!("👤 {} (You, Host)", player_id),
                                    (true, false) => format!("👤 {} (You)", player_id),
                                    (false, true) => format!("👤 {} (Host){}", player_id, bot_type_suffix),
                                    (false, false) => format!("👤 {}{}", player_id, bot_type_suffix),
                                };
                                ui.label(player_display);

                                if player.ready {
                                    ui.label("✅");
                                } else {
                                    ui.label("⏳");
                                }

                                if is_host && !is_self && !is_bot
                                    && ui.button("👁").on_hover_text("Make Observer").clicked() {
                                        cmd_sender.send(ClientCommand::Menu(MenuCommand::MakePlayerObserver {
                                            player_id: player_id.clone(),
                                        }));
                                    }

                                if is_host && !is_self
                                    && ui.button("❌").on_hover_text("Kick").clicked() {
                                        cmd_sender.send(ClientCommand::Menu(MenuCommand::KickFromLobby {
                                            player_id: player_id.clone(),
                                        }));
                                    }
                            });
                        }

                        if !details.observers.is_empty() {
                            ui.add_space(10.0);
                            ui.heading("Observers");
                            ui.add_space(5.0);

                            for observer in &details.observers {
                                ui.horizontal(|ui| {
                                    let is_self = observer.player_id == self.client_id;
                                    let observer_display = if is_self {
                                        format!("👁 {} (You)", observer.player_id)
                                    } else {
                                        format!("👁 {}", observer.player_id)
                                    };

                                    ui.label(observer_display);

                                    if is_host && !is_self
                                        && ui.button("❌").on_hover_text("Kick").clicked() {
                                            cmd_sender.send(ClientCommand::Menu(MenuCommand::KickFromLobby {
                                                player_id: observer.player_id.clone(),
                                            }));
                                        }
                                });
                            }
                        }
                    }
                );
            }
        );

        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                cmd_sender.send(ClientCommand::Menu(MenuCommand::LeaveLobby));
            }
            if !is_self_observer && i.modifiers.ctrl && i.key_pressed(egui::Key::R) {
                cmd_sender.send(ClientCommand::Menu(MenuCommand::MarkReady {
                    ready: !current_ready,
                }));
            }
            if !is_self_observer && i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                cmd_sender.send(ClientCommand::Menu(MenuCommand::BecomeObserver));
            }
            if is_self_observer && !lobby_is_full && i.modifiers.ctrl && i.key_pressed(egui::Key::P) {
                cmd_sender.send(ClientCommand::Menu(MenuCommand::BecomePlayer));
            }
            if can_start && i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                cmd_sender.send(ClientCommand::Menu(MenuCommand::StartGame));
            }
            if is_host && i.modifiers.ctrl && i.key_pressed(egui::Key::B) {
                let bot_type = if is_snake_lobby {
                    crate::state::BotType::Snake(self.selected_snake_bot_type)
                } else if is_tictactoe_lobby {
                    crate::state::BotType::TicTacToe(self.selected_ttt_bot_type)
                } else {
                    return;
                };
                cmd_sender.send(ClientCommand::Menu(MenuCommand::AddBot {
                    bot_type,
                }));
            }
        });

        if !is_offline_lobby {
            self.render_chat_widget(ui, event_log, ChatLocation::InLobby, ChatHeading::Hide);
        }
    }
}

impl eframe::App for MenuApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.shared_state.has_context() {
            self.shared_state.set_context(ctx.clone());
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            if let Some(disconnect_time) = self.disconnecting {
                if disconnect_time.elapsed() < self.disconnect_timeout {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.command_sender.send(ClientCommand::Menu(MenuCommand::Disconnect));
                self.disconnecting = Some(std::time::Instant::now());
            }
        }

        if let Some(disconnect_time) = self.disconnecting
            && disconnect_time.elapsed() >= self.disconnect_timeout {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

        let title = if let Some(ping) = self.shared_state.get_ping() {
            format!("Mini Games - Ping: {}ms", ping)
        } else {
            "Mini Games".to_string()
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

                            if ui.button("Continue Offline").clicked() {
                                self.shared_state.clear_error();
                                self.shared_state.set_connection_mode(crate::state::ConnectionMode::TemporaryOffline);
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
        let current_app_state_type = AppStateType::from(&current_state);

        if self.previous_app_state.as_ref() != Some(&current_app_state_type) {
            let previous = self.previous_app_state;
            self.handle_state_transition(
                &previous,
                &current_app_state_type,
                ctx
            );
            self.previous_app_state = Some(current_app_state_type);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            match current_state {
                AppState::LobbyList { lobbies, chat_messages } => {
                    self.render_lobby_list(ui, ctx, &lobbies, &chat_messages);
                }
                AppState::InLobby { details, event_log } => {
                    self.render_in_lobby(ui, ctx, &details, &event_log);
                }
                AppState::InGame { session_id, game_state, is_observer } => {
                    if self.game_ui.is_none()
                        && let Some(ref state) = game_state {
                            match &state.state {
                                Some(common::game_state_update::State::Snake(_)) => {
                                    self.game_ui = Some(GameUi::new_snake());
                                }
                                Some(common::game_state_update::State::Tictactoe(_)) => {
                                    self.game_ui = Some(GameUi::new_tictactoe());
                                }
                                None => {}
                            }
                        }
                    if let Some(game_ui) = &mut self.game_ui {
                        let is_offline_game = session_id.starts_with("offline_");
                        let sender = if is_offline_game {
                            self.offline_command_sender.as_ref().unwrap_or(&self.command_sender)
                        } else {
                            &self.command_sender
                        };
                        game_ui.render_game(ui, ctx, &session_id, &game_state, &self.client_id, is_observer, sender);
                    }
                }
                AppState::GameOver { scores, winner, last_game_state, game_info, play_again_status, is_observer } => {
                    if self.game_ui.is_none() {
                        match game_info {
                            crate::state::GameEndInfo::Snake(_) => {
                                self.game_ui = Some(GameUi::new_snake());
                            }
                            crate::state::GameEndInfo::TicTacToe(_) => {
                                self.game_ui = Some(GameUi::new_tictactoe());
                            }
                        }
                    }
                    if let Some(game_ui) = &mut self.game_ui {
                        let sender = if self.offline_command_sender.is_some() {
                            self.offline_command_sender.as_ref().unwrap()
                        } else {
                            &self.command_sender
                        };
                        match game_info {
                            crate::state::GameEndInfo::Snake(snake_info) => {
                                game_ui.render_game_over_snake(ui, ctx, &scores, &winner, &self.client_id, &last_game_state, &snake_info, &play_again_status, is_observer, sender);
                            }
                            crate::state::GameEndInfo::TicTacToe(ttt_info) => {
                                game_ui.render_game_over_tictactoe(ui, ctx, &scores, &winner, &self.client_id, &last_game_state, &ttt_info, &play_again_status, is_observer, sender);
                            }
                        }
                    }
                }
            }
        });

        if self.disconnecting.is_some() {
            ctx.request_repaint();
        }
    }
}
