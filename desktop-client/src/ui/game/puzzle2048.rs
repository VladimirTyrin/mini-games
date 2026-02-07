use crate::state::{Puzzle2048GameCommand, PlayAgainStatus};
use crate::CommandSender;
use common::proto::puzzle2048::{Puzzle2048Direction, Puzzle2048GameEndInfo, Puzzle2048GameState, Puzzle2048GameStatus};
use common::{GameStateUpdate, PlayerIdentity, ScoreEntry};
use eframe::egui;
use std::path::PathBuf;

pub struct Puzzle2048GameUi;

impl Puzzle2048GameUi {
    pub fn new() -> Self {
        Self
    }

    pub fn render_game(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        _session_id: &str,
        game_state: &Option<GameStateUpdate>,
        _client_id: &str,
        _is_observer: bool,
        command_sender: &CommandSender,
    ) {
        let Some(state) = game_state else {
            ui.centered_and_justified(|ui| {
                ui.label("Waiting for game state...");
            });
            return;
        };

        let Some(common::game_state_update::State::Puzzle2048(p_state)) = &state.state else {
            ui.label("Invalid game state");
            return;
        };

        self.handle_input(ctx, command_sender, p_state);
        self.render_board(ui, p_state);
    }

    fn handle_input(
        &self,
        ctx: &egui::Context,
        command_sender: &CommandSender,
        state: &Puzzle2048GameState,
    ) {
        if state.status() != Puzzle2048GameStatus::InProgress {
            return;
        }

        let direction = ctx.input(|i| {
            if i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::W) {
                Some(Puzzle2048Direction::Up)
            } else if i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::S) {
                Some(Puzzle2048Direction::Down)
            } else if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::A) {
                Some(Puzzle2048Direction::Left)
            } else if i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::D) {
                Some(Puzzle2048Direction::Right)
            } else {
                None
            }
        });

        if let Some(dir) = direction {
            command_sender.send(crate::state::ClientCommand::Game(
                crate::state::GameCommand::Puzzle2048(Puzzle2048GameCommand::Move {
                    direction: dir,
                }),
            ));
        }
    }

    fn render_board(&self, ui: &mut egui::Ui, state: &Puzzle2048GameState) {
        let width = state.field_width as usize;
        let height = state.field_height as usize;

        ui.vertical_centered(|ui| {
            ui.heading(format!("Score: {}", state.score));
            ui.label(format!("Target: {}", state.target_value));
            ui.add_space(10.0);

            let available = ui.available_size();
            let max_board_size = (available.x.min(available.y - 80.0)).min(500.0);
            let cell_size = (max_board_size / width.max(height) as f32 - 4.0).max(30.0);

            for row in 0..height {
                ui.horizontal(|ui| {
                    ui.add_space(
                        (available.x - (cell_size + 4.0) * width as f32) / 2.0,
                    );
                    for col in 0..width {
                        let idx = row * width + col;
                        let value = state.cells.get(idx).copied().unwrap_or(0);
                        self.render_tile(ui, value, cell_size);
                    }
                });
            }

            ui.add_space(10.0);
            match state.status() {
                Puzzle2048GameStatus::Won => {
                    ui.label(
                        egui::RichText::new("You Win!")
                            .color(egui::Color32::GREEN)
                            .size(24.0),
                    );
                }
                Puzzle2048GameStatus::Lost => {
                    ui.label(
                        egui::RichText::new("Game Over")
                            .color(egui::Color32::RED)
                            .size(24.0),
                    );
                }
                _ => {
                    ui.label("Use Arrow Keys or WASD to move tiles");
                }
            }
        });
    }

    fn render_tile(&self, ui: &mut egui::Ui, value: u32, cell_size: f32) {
        let (bg, text_color) = tile_colors(value);
        let size = egui::vec2(cell_size, cell_size);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());

        ui.painter().rect_filled(rect.shrink(2.0), 6.0, bg);

        if value > 0 {
            let font_size = if value >= 1000 {
                cell_size * 0.25
            } else if value >= 100 {
                cell_size * 0.3
            } else {
                cell_size * 0.4
            };

            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                value.to_string(),
                egui::FontId::proportional(font_size),
                text_color,
            );
        }
    }

    pub fn render_game_over(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        _scores: &[ScoreEntry],
        winner: &Option<PlayerIdentity>,
        client_id: &str,
        last_game_state: &Option<GameStateUpdate>,
        game_info: &Puzzle2048GameEndInfo,
        play_again_status: &PlayAgainStatus,
        is_observer: bool,
        command_sender: &CommandSender,
        replay_path: Option<&PathBuf>,
    ) -> bool {
        let mut watch_replay_clicked = false;

        if let Some(state) = last_game_state
            && let Some(common::game_state_update::State::Puzzle2048(p_state)) = &state.state
        {
            self.render_board(ui, p_state);
        }

        ui.add_space(20.0);

        ui.vertical_centered(|ui| {
            let won = winner.is_some();
            if won {
                ui.heading(
                    egui::RichText::new("You Win!")
                        .color(egui::Color32::GREEN)
                        .size(32.0),
                );
            } else {
                ui.heading(
                    egui::RichText::new("Game Over")
                        .color(egui::Color32::RED)
                        .size(32.0),
                );
            }

            ui.add_space(10.0);

            ui.label(format!("Final Score: {}", game_info.final_score));
            ui.label(format!("Highest Tile: {}", game_info.highest_tile));
            ui.label(format!("Moves Made: {}", game_info.moves_made));

            ui.add_space(20.0);

            if !is_observer {
                match play_again_status {
                    PlayAgainStatus::Available {
                        ready_players,
                        pending_players,
                    } => {
                        let has_voted = ready_players.iter().any(|p| p.player_id == client_id);

                        if !has_voted {
                            if ui.button("Play Again (Enter)").clicked()
                                || ctx.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                command_sender.send(crate::state::ClientCommand::Menu(
                                    crate::state::MenuCommand::PlayAgain,
                                ));
                            }
                        } else {
                            ui.label(format!(
                                "Waiting for others... ({}/{})",
                                ready_players.len(),
                                ready_players.len() + pending_players.len()
                            ));
                        }
                    }
                    PlayAgainStatus::NotAvailable => {
                        ui.label("Play again not available");
                    }
                }
            }

            ui.add_space(10.0);

            if let Some(path) = replay_path {
                if ui.button("Watch Replay").clicked() {
                    watch_replay_clicked = true;
                }
                ui.label(
                    egui::RichText::new(format!("Saved: {}", path.display()))
                        .small()
                        .color(egui::Color32::GRAY),
                );
            }

            if ui.button("Leave (Escape)").clicked()
                || ctx.input(|i| i.key_pressed(egui::Key::Escape))
            {
                command_sender.send(crate::state::ClientCommand::Menu(
                    crate::state::MenuCommand::LeaveLobby,
                ));
            }
        });

        watch_replay_clicked
    }
}

fn tile_colors(value: u32) -> (egui::Color32, egui::Color32) {
    let dark_text = egui::Color32::from_rgb(119, 110, 101);
    let light_text = egui::Color32::WHITE;

    match value {
        0 => (egui::Color32::from_rgb(205, 193, 180), egui::Color32::TRANSPARENT),
        2 => (egui::Color32::from_rgb(238, 228, 218), dark_text),
        4 => (egui::Color32::from_rgb(237, 224, 200), dark_text),
        8 => (egui::Color32::from_rgb(242, 177, 121), light_text),
        16 => (egui::Color32::from_rgb(245, 149, 99), light_text),
        32 => (egui::Color32::from_rgb(246, 124, 95), light_text),
        64 => (egui::Color32::from_rgb(246, 94, 59), light_text),
        128 => (egui::Color32::from_rgb(237, 207, 114), light_text),
        256 => (egui::Color32::from_rgb(237, 204, 97), light_text),
        512 => (egui::Color32::from_rgb(237, 200, 80), light_text),
        1024 => (egui::Color32::from_rgb(237, 197, 63), light_text),
        2048 => (egui::Color32::from_rgb(237, 194, 46), light_text),
        _ => (egui::Color32::from_rgb(60, 58, 50), light_text),
    }
}
