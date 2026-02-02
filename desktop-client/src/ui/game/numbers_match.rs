use crate::state::{NumbersMatchGameCommand, PlayAgainStatus};
use crate::CommandSender;
use common::proto::numbers_match::{
    Cell, GameStatus, HintMode, NumbersMatchGameEndInfo, NumbersMatchGameState,
};
use common::{GameStateUpdate, PlayerIdentity, ScoreEntry};
use eframe::egui;
use std::path::PathBuf;

const FIELD_WIDTH: usize = 9;

pub struct NumbersMatchGameUi {
    selected_index: Option<usize>,
}

impl NumbersMatchGameUi {
    pub fn new() -> Self {
        Self {
            selected_index: None,
        }
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

        let Some(common::game_state_update::State::NumbersMatch(nm_state)) = &state.state else {
            ui.label("Invalid game state");
            return;
        };

        self.render_numbers_match(ui, ctx, nm_state, command_sender);
    }

    fn render_numbers_match(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        state: &NumbersMatchGameState,
        command_sender: &CommandSender,
    ) {
        let game_in_progress = state.status() == GameStatus::InProgress;

        ui.vertical_centered(|ui| {
            self.render_status_bar(ui, state);
            ui.add_space(10.0);
            self.render_game_field(ui, state, game_in_progress, command_sender);
            ui.add_space(10.0);
            self.render_controls(ui, state, game_in_progress, command_sender);
        });

        self.handle_keyboard_input(ctx, state, game_in_progress, command_sender);
    }

    fn render_status_bar(&self, ui: &mut egui::Ui, state: &NumbersMatchGameState) {
        let active_count = state
            .cells
            .iter()
            .filter(|c| c.value > 0 && !c.removed)
            .count();

        ui.horizontal(|ui| {
            ui.label(format!("Cells: {}", active_count));

            ui.separator();

            match state.status() {
                GameStatus::Won => {
                    ui.colored_label(egui::Color32::GREEN, "Victory!");
                }
                GameStatus::Lost => {
                    ui.colored_label(egui::Color32::RED, "No moves left");
                }
                GameStatus::InProgress | GameStatus::Unspecified => {}
            }

            if let Some(hint) = &state.current_hint
                && hint.hint.is_some()
            {
                match &hint.hint {
                    Some(common::proto::numbers_match::hint_result::Hint::SuggestRefill(_)) => {
                        ui.colored_label(egui::Color32::YELLOW, "No pairs - use Refill!");
                    }
                    Some(common::proto::numbers_match::hint_result::Hint::NoMoves(_)) => {
                        ui.colored_label(egui::Color32::RED, "No moves available!");
                    }
                    _ => {}
                }
            }
        });
    }

    fn render_game_field(
        &mut self,
        ui: &mut egui::Ui,
        state: &NumbersMatchGameState,
        game_in_progress: bool,
        command_sender: &CommandSender,
    ) {
        let available_size = ui.available_size();
        let max_cell_size = 44.0_f32;
        let min_cell_size = 24.0_f32;
        let gap = 2.0;

        let row_count = state.row_count as usize;
        let total_gaps_x = (FIELD_WIDTH - 1) as f32 * gap;
        let total_gaps_y = row_count.saturating_sub(1) as f32 * gap;

        let cell_by_width = (available_size.x - total_gaps_x - 40.0) / FIELD_WIDTH as f32;
        let cell_by_height = (available_size.y - total_gaps_y - 150.0) / row_count.max(1) as f32;

        let cell_size = cell_by_width
            .min(cell_by_height)
            .clamp(min_cell_size, max_cell_size);

        let grid_width = FIELD_WIDTH as f32 * cell_size + total_gaps_x;
        let grid_height = row_count as f32 * cell_size + total_gaps_y;

        let hint_indices = self.get_hint_indices(state);

        ui.allocate_ui_with_layout(
            egui::vec2(grid_width + 20.0, grid_height + 20.0),
            egui::Layout::centered_and_justified(egui::Direction::TopDown),
            |ui| {
                egui::Grid::new("numbers_match_grid")
                    .spacing(egui::vec2(gap, gap))
                    .show(ui, |ui| {
                        for row in 0..row_count {
                            for col in 0..FIELD_WIDTH {
                                let index = row * FIELD_WIDTH + col;
                                if index < state.cells.len() {
                                    let cell = &state.cells[index];
                                    self.render_cell(
                                        ui,
                                        cell,
                                        index,
                                        cell_size,
                                        game_in_progress,
                                        hint_indices.contains(&index),
                                        command_sender,
                                    );
                                }
                            }
                            ui.end_row();
                        }
                    });
            },
        );
    }

    fn get_hint_indices(&self, state: &NumbersMatchGameState) -> std::collections::HashSet<usize> {
        let mut indices = std::collections::HashSet::new();
        if let Some(hint) = &state.current_hint
            && let Some(common::proto::numbers_match::hint_result::Hint::Pair(pair)) = &hint.hint
        {
            indices.insert(pair.first_index as usize);
            indices.insert(pair.second_index as usize);
        }
        indices
    }

    fn render_cell(
        &mut self,
        ui: &mut egui::Ui,
        cell: &Cell,
        index: usize,
        cell_size: f32,
        game_in_progress: bool,
        is_hinted: bool,
        command_sender: &CommandSender,
    ) {
        let is_active = cell.value > 0 && !cell.removed;
        let is_selected = self.selected_index == Some(index);

        let (bg_color, text_color) = if is_selected {
            (egui::Color32::from_rgb(180, 180, 50), egui::Color32::WHITE)
        } else if is_hinted && is_active {
            (egui::Color32::from_rgb(50, 150, 50), egui::Color32::WHITE)
        } else if cell.removed && cell.value > 0 {
            (
                egui::Color32::from_gray(40),
                egui::Color32::from_gray(80),
            )
        } else if is_active {
            (egui::Color32::from_gray(60), egui::Color32::WHITE)
        } else {
            (egui::Color32::TRANSPARENT, egui::Color32::TRANSPARENT)
        };

        let size = egui::vec2(cell_size, cell_size);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

        if bg_color != egui::Color32::TRANSPARENT {
            ui.painter()
                .rect_filled(rect, 4.0, bg_color);
        }

        if cell.value > 0 {
            let font_size = if cell_size >= 40.0 {
                20.0
            } else if cell_size >= 32.0 {
                16.0
            } else {
                14.0
            };

            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                cell.value.to_string(),
                egui::FontId::proportional(font_size),
                text_color,
            );
        }

        if response.clicked() && game_in_progress && is_active {
            if let Some(first_index) = self.selected_index {
                if first_index != index {
                    command_sender.send(crate::state::ClientCommand::Game(
                        crate::state::GameCommand::NumbersMatch(
                            NumbersMatchGameCommand::RemovePair {
                                first_index: first_index as u32,
                                second_index: index as u32,
                            },
                        ),
                    ));
                }
                self.selected_index = None;
            } else {
                self.selected_index = Some(index);
            }
        }
    }

    fn render_controls(
        &mut self,
        ui: &mut egui::Ui,
        state: &NumbersMatchGameState,
        game_in_progress: bool,
        command_sender: &CommandSender,
    ) {
        ui.horizontal(|ui| {
            let can_refill = game_in_progress && state.refills_remaining > 0;
            let refill_text = format!("Refill [{}] (Ctrl+F)", state.refills_remaining);

            if ui
                .add_enabled(can_refill, egui::Button::new(&refill_text))
                .clicked()
            {
                command_sender.send(crate::state::ClientCommand::Game(
                    crate::state::GameCommand::NumbersMatch(NumbersMatchGameCommand::Refill),
                ));
                self.selected_index = None;
            }

            ui.separator();

            let has_active_hint = state.current_hint.is_some();
            let can_use_hint = game_in_progress
                && !has_active_hint
                && state.hint_mode() != HintMode::Disabled
                && (state.hint_mode() == HintMode::Unlimited
                    || state.hints_remaining.unwrap_or(0) > 0);

            let hints_display = if state.hint_mode() == HintMode::Unlimited {
                "∞".to_string()
            } else if state.hint_mode() == HintMode::Disabled {
                "-".to_string()
            } else {
                state.hints_remaining.unwrap_or(0).to_string()
            };

            let hint_text = format!("Hint [{}] (Ctrl+H)", hints_display);

            if ui
                .add_enabled(can_use_hint, egui::Button::new(&hint_text))
                .clicked()
            {
                command_sender.send(crate::state::ClientCommand::Game(
                    crate::state::GameCommand::NumbersMatch(NumbersMatchGameCommand::RequestHint),
                ));
                self.selected_index = None;
            }
        });
    }

    fn handle_keyboard_input(
        &mut self,
        ctx: &egui::Context,
        state: &NumbersMatchGameState,
        game_in_progress: bool,
        command_sender: &CommandSender,
    ) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                self.selected_index = None;
            }

            if game_in_progress && i.modifiers.ctrl && i.key_pressed(egui::Key::F)
                && state.refills_remaining > 0
            {
                command_sender.send(crate::state::ClientCommand::Game(
                    crate::state::GameCommand::NumbersMatch(NumbersMatchGameCommand::Refill),
                ));
                self.selected_index = None;
            }

            if game_in_progress && i.modifiers.ctrl && i.key_pressed(egui::Key::H) {
                let has_active_hint = state.current_hint.is_some();
                let can_use_hint = !has_active_hint
                    && state.hint_mode() != HintMode::Disabled
                    && (state.hint_mode() == HintMode::Unlimited
                        || state.hints_remaining.unwrap_or(0) > 0);

                if can_use_hint {
                    command_sender.send(crate::state::ClientCommand::Game(
                        crate::state::GameCommand::NumbersMatch(
                            NumbersMatchGameCommand::RequestHint,
                        ),
                    ));
                    self.selected_index = None;
                }
            }
        });
    }

    pub fn render_game_over(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        scores: &[ScoreEntry],
        winner: &Option<PlayerIdentity>,
        client_id: &str,
        last_game_state: &Option<GameStateUpdate>,
        game_info: &NumbersMatchGameEndInfo,
        play_again_status: &PlayAgainStatus,
        is_observer: bool,
        command_sender: &CommandSender,
        replay_path: Option<&PathBuf>,
    ) -> bool {
        let mut watch_replay_clicked = false;

        if let Some(state) = last_game_state
            && let Some(common::game_state_update::State::NumbersMatch(nm_state)) = &state.state
        {
            self.render_numbers_match_static(ui, nm_state);
        }

        ui.add_space(20.0);

        ui.vertical_centered(|ui| {
            let won = winner.is_some();
            if won {
                ui.heading(egui::RichText::new("Victory!").color(egui::Color32::GREEN).size(32.0));
            } else {
                ui.heading(egui::RichText::new("Game Over").color(egui::Color32::RED).size(32.0));
            }

            ui.add_space(10.0);

            ui.label(format!("Pairs removed: {}", game_info.pairs_removed));
            ui.label(format!("Refills used: {}", game_info.refills_used));
            ui.label(format!("Hints used: {}", game_info.hints_used));

            ui.add_space(20.0);

            if !is_observer {
                match play_again_status {
                    PlayAgainStatus::Available { ready_players, pending_players } => {
                        let has_voted = ready_players
                            .iter()
                            .any(|p| p.player_id == client_id);

                        if !has_voted {
                            if ui.button("Play Again (Enter)").clicked() {
                                command_sender.send(crate::state::ClientCommand::Menu(
                                    crate::state::MenuCommand::PlayAgain,
                                ));
                            }
                        } else {
                            ui.label("Waiting for others...");
                        }

                        ui.add_space(5.0);
                        ui.label(format!(
                            "Ready: {} / {}",
                            ready_players.len(),
                            ready_players.len() + pending_players.len()
                        ));
                    }
                    PlayAgainStatus::NotAvailable => {
                        ui.label("Play again not available");
                    }
                }
            }

            ui.add_space(10.0);

            if ui.button("Leave (Esc)").clicked() {
                command_sender.send(crate::state::ClientCommand::Menu(
                    crate::state::MenuCommand::LeaveLobby,
                ));
            }

            if replay_path.is_some() {
                ui.add_space(5.0);
                if ui.button("Watch Replay").clicked() {
                    watch_replay_clicked = true;
                }
            }
        });

        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                command_sender.send(crate::state::ClientCommand::Menu(
                    crate::state::MenuCommand::LeaveLobby,
                ));
            }
            if i.key_pressed(egui::Key::Enter) && !is_observer
                && let PlayAgainStatus::Available { ready_players, .. } = play_again_status
            {
                let has_voted = ready_players.iter().any(|p| p.player_id == client_id);
                if !has_voted {
                    command_sender.send(crate::state::ClientCommand::Menu(
                        crate::state::MenuCommand::PlayAgain,
                    ));
                }
            }
        });

        let _ = scores;

        watch_replay_clicked
    }

    fn render_numbers_match_static(&self, ui: &mut egui::Ui, state: &NumbersMatchGameState) {
        let cell_size = 30.0;
        let gap = 2.0;
        let row_count = state.row_count as usize;

        ui.vertical_centered(|ui| {
            egui::Grid::new("numbers_match_static_grid")
                .spacing(egui::vec2(gap, gap))
                .show(ui, |ui| {
                    for row in 0..row_count {
                        for col in 0..FIELD_WIDTH {
                            let index = row * FIELD_WIDTH + col;
                            if index < state.cells.len() {
                                let cell = &state.cells[index];
                                self.render_static_cell(ui, cell, cell_size);
                            }
                        }
                        ui.end_row();
                    }
                });
        });
    }

    fn render_static_cell(&self, ui: &mut egui::Ui, cell: &Cell, cell_size: f32) {
        let (bg_color, text_color) = if cell.removed && cell.value > 0 {
            (egui::Color32::from_gray(40), egui::Color32::from_gray(80))
        } else if cell.value > 0 {
            (egui::Color32::from_gray(60), egui::Color32::WHITE)
        } else {
            (egui::Color32::TRANSPARENT, egui::Color32::TRANSPARENT)
        };

        let size = egui::vec2(cell_size, cell_size);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());

        if bg_color != egui::Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, 4.0, bg_color);
        }

        if cell.value > 0 {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                cell.value.to_string(),
                egui::FontId::proportional(14.0),
                text_color,
            );
        }
    }
}
