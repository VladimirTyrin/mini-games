use super::colors::generate_color_from_client_id;
use super::game::GameUi;
use super::sprites::Sprites;
use crate::config::{Config, GameType, NumbersMatchLobbyConfig, SnakeLobbyConfig};
use crate::state::{AppState, MenuCommand, ClientCommand, SharedState, LobbyConfig, ReplayInfo};
use crate::replay_playback::{ReplayCommand, run_replay_playback};
use crate::CommandSender;
use common::config::{ConfigManager, FileContentConfigProvider, YamlConfigSerializer};
use common::{proto::snake::{Direction, SnakeBotType}, WallCollisionMode, ReplayGame, log};
use common::{LobbyDetails, LobbyInfo};
use common::replay::{load_replay_metadata, REPLAY_FILE_EXTENSION};
use eframe::egui;
use egui::{Align, Layout};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use std::path::PathBuf;
use tokio::sync::mpsc;

type ClientConfigManager = ConfigManager<FileContentConfigProvider, Config, YamlConfigSerializer>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppStateType {
    LobbyList,
    InLobby,
    InGame,
    GameOver,
    ReplayList,
    WatchingReplay,
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
            AppState::ReplayList { .. } => Self::ReplayList,
            AppState::WatchingReplay { .. } => Self::WatchingReplay,
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
    replay_command_sender: Option<mpsc::UnboundedSender<ReplayCommand>>,
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
    selected_hint_mode: common::proto::numbers_match::HintMode,
    target_value_input: String,
    disconnect_timeout: std::time::Duration,
    disconnecting: Option<std::time::Instant>,
    game_ui: Option<GameUi>,
    config_manager: ClientConfigManager,
    server_address_input: String,
    sprites: Sprites,
    chat_input: String,
    normal_window_size: Option<egui::Vec2>,
    previous_app_state: Option<AppStateType>,
    replay_speed: f32,
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
            replay_command_sender: None,
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
            selected_hint_mode: config.numbers_match.hint_mode,
            target_value_input: config.puzzle2048.target_value.to_string(),
            disconnecting: None,
            disconnect_timeout,
            game_ui: None,
            config_manager,
            server_address_input: String::new(),
            sprites: Sprites::load(),
            chat_input: String::new(),
            normal_window_size: None,
            previous_app_state: None,
            replay_speed: 1.0,
        }
    }

    fn should_maximize_for_game(game_type: GameType) -> bool {
        match game_type {
            GameType::Snake => true,
            GameType::TicTacToe => true,
            GameType::NumbersMatch => false,
            GameType::Puzzle2048 => false,
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
                if Self::should_maximize_for_game(self.selected_game_type) {
                    if self.normal_window_size.is_none() {
                        self.normal_window_size = Some(
                            ctx.input(|i| i.viewport().inner_rect)
                                .map(|r| r.size())
                                .unwrap_or(egui::vec2(600.0, 700.0))
                        );
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                }
            }

            (_, AppStateType::WatchingReplay) => {
                if self.normal_window_size.is_none() {
                    self.normal_window_size = Some(
                        ctx.input(|i| i.viewport().inner_rect)
                            .map(|r| r.size())
                            .unwrap_or(egui::vec2(600.0, 700.0))
                    );
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }

            (Some(AppStateType::GameOver), AppStateType::LobbyList) |
            (Some(AppStateType::WatchingReplay), AppStateType::ReplayList) => {
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

        if let Some(response) = response_opt
            && response.lost_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter))
        {
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

            ui.add_space(10.0);

            if ui.button("📼 Watch Replays (Ctrl+R)").clicked() {
                self.open_replay_list();
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
            if i.modifiers.ctrl && i.key_pressed(egui::Key::R) {
                self.open_replay_list();
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

                    if ui.button("📼 Replays (Ctrl+R)").clicked() {
                        self.open_replay_list();
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
                    if i.modifiers.ctrl && i.key_pressed(egui::Key::R) {
                        self.open_replay_list();
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

                            let (game_icon, game_settings) = match lobby.settings.as_ref().and_then(|s| s.settings.as_ref()) {
                                Some(common::lobby_settings::Settings::Snake(s)) => {
                                    let wall_mode = match common::proto::snake::WallCollisionMode::try_from(s.wall_collision_mode) {
                                        Ok(common::proto::snake::WallCollisionMode::WrapAround) => "wrap",
                                        _ => "death",
                                    };
                                    ("🐍", format!("{}x{}, {}", s.field_width, s.field_height, wall_mode))
                                }
                                Some(common::lobby_settings::Settings::Tictactoe(s)) => {
                                    ("⭕", format!("{}x{}, {} to win", s.field_width, s.field_height, s.win_count))
                                }
                                Some(common::lobby_settings::Settings::NumbersMatch(_)) => {
                                    ("🔢", "Single player puzzle".to_string())
                                }
                                Some(common::lobby_settings::Settings::StackAttack(_)) => {
                                    ("📦", "Cooperative puzzle".to_string())
                                }
                                Some(common::lobby_settings::Settings::Puzzle2048(s)) => {
                                    ("🧩", format!("{}x{}, target {}", s.field_width, s.field_height, s.target_value))
                                }
                                None => ("❓", "Unknown".to_string()),
                            };

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
                                                ui.label(format!("{} {}", game_icon, lobby.lobby_name));
                                                ui.label(format!(
                                                    "👥 {}/{}{} | {}",
                                                    lobby.current_players, lobby.max_players, full_message, game_settings
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
                    if ui.radio_value(&mut self.selected_game_type, GameType::NumbersMatch, "Numbers Match").clicked() {
                        self.max_players_input = "1".to_string();
                    }
                    if ui.radio_value(&mut self.selected_game_type, GameType::Puzzle2048, "2048").clicked() {
                        let config = self.config_manager.get_config().unwrap();
                        self.max_players_input = "1".to_string();
                        self.field_width_input = config.puzzle2048.field_width.to_string();
                        self.field_height_input = config.puzzle2048.field_height.to_string();
                        self.target_value_input = config.puzzle2048.target_value.to_string();
                    }
                });

                ui.separator();

                ui.label("Lobby Name:");
                ui.text_edit_singleline(&mut self.lobby_name_input);

                let is_single_player_puzzle = self.selected_game_type == GameType::NumbersMatch
                    || self.selected_game_type == GameType::Puzzle2048;

                if !is_single_player_puzzle {
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
                }

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
                    GameType::NumbersMatch => {
                        ui.label("Hint Mode:");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.selected_hint_mode, common::proto::numbers_match::HintMode::Limited, "Limited");
                            ui.radio_value(&mut self.selected_hint_mode, common::proto::numbers_match::HintMode::Unlimited, "Unlimited");
                            ui.radio_value(&mut self.selected_hint_mode, common::proto::numbers_match::HintMode::Disabled, "Disabled");
                        });
                    }
                    GameType::Puzzle2048 => {
                        ui.label("Field Width:");
                        ui.text_edit_singleline(&mut self.field_width_input);
                        ui.label("Field Height:");
                        ui.text_edit_singleline(&mut self.field_height_input);
                        ui.label("Target Value:");
                        ui.text_edit_singleline(&mut self.target_value_input);
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
            let lobby_config = match self.selected_game_type {
                GameType::Snake => {
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
                    let Some(field_width) = parse_u32_input(&self.field_width_input, "Field width", &self.shared_state) else {
                        return;
                    };
                    let Some(field_height) = parse_u32_input(&self.field_height_input, "Field height", &self.shared_state) else {
                        return;
                    };
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
                GameType::NumbersMatch => {
                    let nm_config = NumbersMatchLobbyConfig {
                        hint_mode: self.selected_hint_mode,
                    };

                    let mut config = self.config_manager.get_config().unwrap();
                    config.numbers_match = nm_config;
                    config.last_game = Some(GameType::NumbersMatch);
                    self.config_manager.set_config(&config).ok();

                    LobbyConfig::NumbersMatch(nm_config)
                }
                GameType::Puzzle2048 => {
                    let Some(field_width) = parse_u32_input(&self.field_width_input, "Field width", &self.shared_state) else {
                        return;
                    };
                    let Some(field_height) = parse_u32_input(&self.field_height_input, "Field height", &self.shared_state) else {
                        return;
                    };
                    let Some(target_value) = parse_u32_input(&self.target_value_input, "Target value", &self.shared_state) else {
                        return;
                    };

                    let p_config = crate::config::Puzzle2048LobbyConfig {
                        field_width,
                        field_height,
                        target_value,
                    };

                    let mut config = self.config_manager.get_config().unwrap();
                    config.puzzle2048 = p_config;
                    config.last_game = Some(GameType::Puzzle2048);
                    self.config_manager.set_config(&config).ok();

                    LobbyConfig::Puzzle2048(p_config)
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
            !details.players.is_empty()
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

    fn open_replay_list(&mut self) {
        let config = self.config_manager.get_config().expect("Failed to load config");
        let replay_dir = PathBuf::from(&config.replays.location);
        let replays = self.load_replays_from_directory(&replay_dir);
        self.shared_state.set_state_from_ui(AppState::ReplayList { replays });
    }

    fn load_replays_from_directory(&self, dir: &PathBuf) -> Vec<ReplayInfo> {
        let mut replays = Vec::new();

        let Ok(entries) = std::fs::read_dir(dir) else {
            return replays;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension()
                && ext == REPLAY_FILE_EXTENSION
            {
                match load_replay_metadata(&path) {
                    Ok(metadata) => {
                        let Ok(game) = ReplayGame::try_from(metadata.game) else {
                            common::log!("Skipping replay with unknown game type {}: {}", metadata.game, path.display());
                            continue;
                        };
                        if game == ReplayGame::Unspecified {
                            common::log!("Skipping replay with unspecified game type: {}", path.display());
                            continue;
                        }

                        let players: Vec<String> = metadata.players.iter()
                            .map(|p| {
                                if p.is_bot {
                                    format!("{} (Bot)", p.player_id)
                                } else {
                                    p.player_id.clone()
                                }
                            })
                            .collect();

                        replays.push(ReplayInfo {
                            file_path: path,
                            game,
                            timestamp_ms: metadata.game_started_timestamp_ms,
                            players,
                            engine_version: metadata.engine_version,
                        });
                    }
                    Err(e) => {
                        common::log!("Failed to load replay metadata from {}: {}", path.display(), e);
                    }
                }
            }
        }

        replays.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
        replays
    }

    fn render_replay_list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, replays: &[ReplayInfo]) {
        ui.heading("📼 Replays");
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("⬅ Back (Esc)").clicked() {
                self.back_to_lobby_list();
            }

            if ui.button("🔄 Refresh (F5)").clicked() {
                self.open_replay_list();
            }

            if ui.button("📂 Open File... (Ctrl+O)").clicked() {
                self.open_replay_file_dialog();
            }
        });

        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                self.back_to_lobby_list();
            }
            if i.key_pressed(egui::Key::F5) {
                self.open_replay_list();
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                self.open_replay_file_dialog();
            }
        });

        ui.separator();

        if replays.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("No replays found.");
                ui.add_space(10.0);
                let config = self.config_manager.get_config().expect("Failed to load config");
                ui.label(format!("Replay directory: {}", config.replays.location));
            });
        } else {
            egui::ScrollArea::vertical()
                .id_salt("replay_list_scroll")
                .show(ui, |ui| {
                    for (index, replay) in replays.iter().enumerate() {
                        let game_icon = match replay.game {
                            ReplayGame::Snake => "🐍",
                            ReplayGame::Tictactoe => "⭕",
                            ReplayGame::NumbersMatch => "🔢",
                            ReplayGame::StackAttack => "📦",
                            ReplayGame::Puzzle2048 => "🧩",
                            ReplayGame::Unspecified => "❓",
                        };

                        let game_name = match replay.game {
                            ReplayGame::Snake => "Snake",
                            ReplayGame::Tictactoe => "TicTacToe",
                            ReplayGame::NumbersMatch => "Numbers Match",
                            ReplayGame::StackAttack => "Stack Attack",
                            ReplayGame::Puzzle2048 => "2048",
                            ReplayGame::Unspecified => "Unknown",
                        };

                        let datetime = chrono::DateTime::from_timestamp_millis(replay.timestamp_ms)
                            .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_else(|| "Unknown date".to_string());

                        let players_str = replay.players.join(", ");

                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 70.0),
                            egui::Sense::click(),
                        );

                        let is_selected = index == 0;
                        let bg_color = if response.hovered() {
                            egui::Color32::from_gray(60)
                        } else if is_selected {
                            egui::Color32::from_gray(50)
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        ui.painter().rect_filled(rect, 4.0, bg_color);

                        ui.scope_builder(
                            egui::UiBuilder::new().max_rect(rect.shrink(8.0)),
                            |ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(format!("{} {}", game_icon, game_name)).strong());
                                        ui.label(format!("📅 {}", datetime));
                                        ui.label(format!("👥 {}", players_str));
                                    });

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(egui::RichText::new(format!("v{}", replay.engine_version)).small().color(egui::Color32::GRAY));
                                    });
                                });
                            }
                        );

                        if response.double_clicked() {
                            self.play_replay(&replay.file_path);
                        }

                        if response.clicked() {
                            // Single click could select, double click plays
                        }
                    }
                });
        }
    }

    fn back_to_lobby_list(&mut self) {
        let is_offline = self.shared_state.get_connection_mode() == crate::state::ConnectionMode::TemporaryOffline;
        self.shared_state.set_state_from_ui(AppState::LobbyList {
            lobbies: vec![],
            chat_messages: AllocRingBuffer::new(crate::constants::CHAT_BUFFER_SIZE),
        });
        if !is_offline {
            self.command_sender.send(ClientCommand::Menu(MenuCommand::ListLobbies));
        }
    }

    fn open_replay_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Replay files", &[REPLAY_FILE_EXTENSION])
            .pick_file()
        {
            self.play_replay(&path);
        }
    }

    pub fn open_replay_file(&mut self, path: &std::path::Path) {
        self.play_replay(path);
    }

    fn play_replay(&mut self, path: &std::path::Path) {
        let (tx, rx) = mpsc::unbounded_channel();
        self.replay_command_sender = Some(tx);
        self.game_ui = None;

        let file_path = path.to_path_buf();
        let shared_state = self.shared_state.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                run_replay_playback(file_path, shared_state, rx).await;
            });
        });
    }

    fn stop_replay(&mut self) {
        if let Some(sender) = self.replay_command_sender.take()
            && let Err(e) = sender.send(ReplayCommand::Stop)
        {
            log!("[replay] Failed to send stop command: {}", e);
        }
        self.game_ui = None;
    }

    fn render_watching_replay(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        game_state: &Option<common::GameStateUpdate>,
        is_paused: bool,
        current_tick: u64,
        total_ticks: u64,
        replay_version: &str,
        is_finished: bool,
        highlighted_pair: Option<(u32, u32)>,
    ) {
        if self.game_ui.is_none()
            && let Some(state) = game_state
        {
            match &state.state {
                Some(common::game_state_update::State::Snake(_)) => {
                    self.game_ui = Some(GameUi::new_snake());
                }
                Some(common::game_state_update::State::Tictactoe(_)) => {
                    self.game_ui = Some(GameUi::new_tictactoe());
                }
                Some(common::game_state_update::State::NumbersMatch(_)) => {
                    self.game_ui = Some(GameUi::new_numbers_match());
                }
                Some(common::game_state_update::State::StackAttack(_)) => {
                    // Stack Attack replay not yet implemented
                }
                Some(common::game_state_update::State::Puzzle2048(_)) => {
                    self.game_ui = Some(GameUi::new_puzzle2048());
                }
                None => {}
            }
        }

        let version_mismatch = replay_version != common::version::VERSION;

        ui.horizontal(|ui| {
            if ui.button("⬅ Back (Esc)").clicked() {
                self.stop_replay();
                self.open_replay_list();
            }

            ui.separator();

            if is_paused {
                if ui.button("▶ Play (Space)").clicked()
                    && let Some(ref sender) = self.replay_command_sender
                    && let Err(e) = sender.send(ReplayCommand::Resume)
                {
                    log!("[replay] Failed to send resume command: {}", e);
                }
            } else if ui.button("⏸ Pause (Space)").clicked()
                && let Some(ref sender) = self.replay_command_sender
                && let Err(e) = sender.send(ReplayCommand::Pause)
            {
                log!("[replay] Failed to send pause command: {}", e);
            }

            ui.separator();

            ui.label("Speed:");
            let speed_options = [0.5, 1.0, 2.0, 4.0];
            for speed in speed_options {
                let label = format!("{}x", speed);
                let selected = (self.replay_speed - speed).abs() < 0.01;
                if ui.selectable_label(selected, &label).clicked() {
                    self.replay_speed = speed;
                    if let Some(ref sender) = self.replay_command_sender
                        && let Err(e) = sender.send(ReplayCommand::SetSpeed(speed))
                    {
                        log!("[replay] Failed to send speed command: {}", e);
                    }
                }
            }

            ui.separator();

            let progress = if total_ticks > 0 {
                current_tick as f32 / total_ticks as f32
            } else {
                0.0
            };
            ui.label(format!("Progress: {}/{} ({:.0}%)", current_tick, total_ticks, progress * 100.0));

            ui.separator();

            if version_mismatch {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    format!("Replay v{} / Client v{}", replay_version, common::version::VERSION)
                );
            } else {
                ui.label(format!("v{}", replay_version));
            }

            if is_finished {
                ui.separator();
                if ui.button("🔄 Watch Again (R)").clicked()
                    && let Some(ref sender) = self.replay_command_sender
                    && let Err(e) = sender.send(ReplayCommand::Restart)
                {
                    log!("[replay] Failed to send restart command: {}", e);
                }
            }
        });

        if version_mismatch {
            ui.colored_label(
                egui::Color32::YELLOW,
                "Warning: Replay was recorded with a different version. Playback issues may occur."
            );
        }

        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                self.stop_replay();
                self.open_replay_list();
            }
            if i.key_pressed(egui::Key::Space)
                && let Some(ref sender) = self.replay_command_sender
            {
                let cmd = if is_paused {
                    ReplayCommand::Resume
                } else {
                    ReplayCommand::Pause
                };
                if let Err(e) = sender.send(cmd) {
                    log!("[replay] Failed to send pause/resume command: {}", e);
                }
            }
            if is_finished
                && i.key_pressed(egui::Key::R)
                && let Some(ref sender) = self.replay_command_sender
                && let Err(e) = sender.send(ReplayCommand::Restart)
            {
                log!("[replay] Failed to send restart command: {}", e);
            }
        });

        ui.separator();

        if let Some(game_ui) = &mut self.game_ui {
            let dummy_sender = CommandSender::Grpc(mpsc::unbounded_channel().0);
            game_ui.render_game(ui, ctx, "replay", game_state, &self.client_id, true, &dummy_sender, is_finished, highlighted_pair);
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Loading replay...");
            });
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
                                Some(common::game_state_update::State::NumbersMatch(_)) => {
                                    self.game_ui = Some(GameUi::new_numbers_match());
                                }
                                Some(common::game_state_update::State::StackAttack(_)) => {
                                    // Stack Attack desktop client not yet implemented
                                }
                                Some(common::game_state_update::State::Puzzle2048(_)) => {
                                    self.game_ui = Some(GameUi::new_puzzle2048());
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
                        game_ui.render_game(ui, ctx, &session_id, &game_state, &self.client_id, is_observer, sender, false, None);
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
                            crate::state::GameEndInfo::NumbersMatch(_) => {
                                self.game_ui = Some(GameUi::new_numbers_match());
                            }
                            crate::state::GameEndInfo::StackAttack(_) => {
                                // Stack Attack desktop client not yet implemented
                            }
                            crate::state::GameEndInfo::Puzzle2048(_) => {
                                self.game_ui = Some(GameUi::new_puzzle2048());
                            }
                        }
                    }
                    let replay_path = self.shared_state.get_last_replay_path();
                    let mut watch_replay_clicked = false;
                    if let Some(game_ui) = &mut self.game_ui {
                        let sender = if self.offline_command_sender.is_some() {
                            self.offline_command_sender.as_ref().unwrap()
                        } else {
                            &self.command_sender
                        };
                        watch_replay_clicked = match game_info {
                            crate::state::GameEndInfo::Snake(snake_info) => {
                                game_ui.render_game_over_snake(ui, ctx, &scores, &winner, &self.client_id, &last_game_state, &snake_info, &play_again_status, is_observer, sender, replay_path.as_ref())
                            }
                            crate::state::GameEndInfo::TicTacToe(ttt_info) => {
                                game_ui.render_game_over_tictactoe(ui, ctx, &scores, &winner, &self.client_id, &last_game_state, &ttt_info, &play_again_status, is_observer, sender, replay_path.as_ref())
                            }
                            crate::state::GameEndInfo::NumbersMatch(nm_info) => {
                                game_ui.render_game_over_numbers_match(ui, ctx, &scores, &winner, &self.client_id, &last_game_state, &nm_info, &play_again_status, is_observer, sender, replay_path.as_ref())
                            }
                            crate::state::GameEndInfo::StackAttack(_) => {
                                false
                            }
                            crate::state::GameEndInfo::Puzzle2048(p_info) => {
                                game_ui.render_game_over_puzzle2048(ui, ctx, &scores, &winner, &self.client_id, &last_game_state, &p_info, &play_again_status, is_observer, sender, replay_path.as_ref())
                            }
                        };
                    }
                    if watch_replay_clicked
                        && let Some(path) = replay_path
                    {
                        let sender = if self.offline_command_sender.is_some() {
                            self.offline_command_sender.as_ref().unwrap()
                        } else {
                            &self.command_sender
                        };
                        sender.send(ClientCommand::Menu(MenuCommand::LeaveLobby));
                        self.play_replay(&path);
                    }
                }
                AppState::ReplayList { replays } => {
                    self.render_replay_list(ui, ctx, &replays);
                }
                AppState::WatchingReplay { game_state, is_paused, current_tick, total_ticks, replay_version, is_finished, highlighted_pair } => {
                    self.render_watching_replay(ui, ctx, &game_state, is_paused, current_tick, total_ticks, &replay_version, is_finished, highlighted_pair);
                }
            }
        });

        if self.disconnecting.is_some() {
            ctx.request_repaint();
        }
    }
}
