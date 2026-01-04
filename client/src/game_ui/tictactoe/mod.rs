use crate::state::{ClientCommand, GameCommand, MenuCommand, PlayAgainStatus, TicTacToeGameCommand};
use crate::colors::generate_color_from_client_id;
use common::{proto::tictactoe::TicTacToeGameEndReason, GameStateUpdate, ScoreEntry, PlayerIdentity};
use eframe::egui;
use tokio::sync::mpsc;

pub struct TicTacToeGameUi {
    last_hover: Option<(u32, u32)>,
}

impl TicTacToeGameUi {
    const BOARD_PADDING: f32 = 40.0;
    const INFO_PANEL_WIDTH: f32 = 200.0;
    const MIN_CELL_SIZE: f32 = 30.0;
    const MAX_CELL_SIZE: f32 = 100.0;
    const LINE_WIDTH: f32 = 2.0;

    pub fn new() -> Self {
        Self {
            last_hover: None,
        }
    }

    fn calculate_cell_size(
        available_width: f32,
        available_height: f32,
        field_width: u32,
        field_height: u32,
    ) -> f32 {
        let available_board_width = available_width - Self::INFO_PANEL_WIDTH - (Self::BOARD_PADDING * 2.0);
        let available_board_height = available_height - (Self::BOARD_PADDING * 2.0);

        let cell_width = available_board_width / field_width as f32;
        let cell_height = available_board_height / field_height as f32;

        let cell_size = cell_width.min(cell_height);

        cell_size.clamp(Self::MIN_CELL_SIZE, Self::MAX_CELL_SIZE)
    }

    pub fn render_game(
        &mut self,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
        _session_id: &str,
        game_state: &Option<GameStateUpdate>,
        client_id: &str,
        command_tx: &mpsc::UnboundedSender<ClientCommand>,
    ) {
        let Some(game_state_update) = game_state else {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Waiting for game to start...");
                    ui.spinner();
                });
            });
            return;
        };

        let state = match &game_state_update.state {
            Some(common::game_state_update::State::Tictactoe(ttt_state)) => ttt_state,
            _ => {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Waiting for game to start...");
                        ui.spinner();
                    });
                });
                return;
            }
        };

        let available_width = ui.available_width();
        let available_height = ui.available_height();

        let cell_size = Self::calculate_cell_size(
            available_width,
            available_height,
            state.field_width,
            state.field_height,
        );

        let board_width = cell_size * state.field_width as f32;
        let _board_height = cell_size * state.field_height as f32;

        ui.horizontal(|ui| {
            ui.allocate_ui(
                egui::vec2(board_width + Self::BOARD_PADDING * 2.0, available_height),
                |ui| {
                    self.render_board(ui, state, cell_size, client_id, command_tx);
                },
            );

            ui.separator();

            ui.vertical(|ui| {
                self.render_info_panel(ui, state, client_id);
            });
        });
    }

    fn render_board(
        &mut self,
        ui: &mut egui::Ui,
        state: &common::proto::tictactoe::TicTacToeGameState,
        cell_size: f32,
        client_id: &str,
        command_tx: &mpsc::UnboundedSender<ClientCommand>,
    ) {
        let board_width = cell_size * state.field_width as f32;
        let board_height = cell_size * state.field_height as f32;

        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(board_width, board_height),
            egui::Sense::click(),
        );

        let painter = ui.painter();

        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(240, 240, 240));

        for i in 0..=state.field_width {
            let x = rect.left() + i as f32 * cell_size;
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(Self::LINE_WIDTH, egui::Color32::BLACK),
            );
        }

        for i in 0..=state.field_height {
            let y = rect.top() + i as f32 * cell_size;
            painter.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                egui::Stroke::new(Self::LINE_WIDTH, egui::Color32::BLACK),
            );
        }

        let is_my_turn = state.current_player.as_ref()
            .map(|p| p.player_id == client_id)
            .unwrap_or(false);

        for cell_mark in &state.board {
            let x = cell_mark.x;
            let y = cell_mark.y;

            let cell_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + x as f32 * cell_size,
                    rect.top() + y as f32 * cell_size,
                ),
                egui::vec2(cell_size, cell_size),
            );

            match cell_mark.mark {
                2 => self.draw_x(painter, cell_rect),
                3 => self.draw_o(painter, cell_rect),
                _ => {}
            }
        }

        if is_my_turn && state.status == 1 {
            if let Some(hover_pos) = response.hover_pos() {
                let x = ((hover_pos.x - rect.left()) / cell_size) as u32;
                let y = ((hover_pos.y - rect.top()) / cell_size) as u32;

                if x < state.field_width && y < state.field_height {
                    let is_empty = !state.board.iter().any(|cell| {
                        cell.x == x && cell.y == y && cell.mark != 1
                    });

                    if is_empty {
                        let hover_rect = egui::Rect::from_min_size(
                            egui::pos2(
                                rect.left() + x as f32 * cell_size,
                                rect.top() + y as f32 * cell_size,
                            ),
                            egui::vec2(cell_size, cell_size),
                        );

                        painter.rect_filled(
                            hover_rect,
                            0.0,
                            egui::Color32::from_rgba_unmultiplied(100, 150, 255, 50),
                        );

                        self.last_hover = Some((x, y));
                    } else {
                        self.last_hover = None;
                    }
                } else {
                    self.last_hover = None;
                }
            } else {
                self.last_hover = None;
            }

            if response.clicked() {
                if let Some((x, y)) = self.last_hover {
                    let _ = command_tx.send(ClientCommand::Game(GameCommand::TicTacToe(TicTacToeGameCommand::PlaceMark { x, y })));
                }
            }
        }
    }

    fn draw_x(&self, painter: &egui::Painter, rect: egui::Rect) {
        let padding = rect.width() * 0.2;
        let color = egui::Color32::from_rgb(220, 50, 50);
        let stroke = egui::Stroke::new(4.0, color);

        painter.line_segment(
            [
                egui::pos2(rect.left() + padding, rect.top() + padding),
                egui::pos2(rect.right() - padding, rect.bottom() - padding),
            ],
            stroke,
        );

        painter.line_segment(
            [
                egui::pos2(rect.right() - padding, rect.top() + padding),
                egui::pos2(rect.left() + padding, rect.bottom() - padding),
            ],
            stroke,
        );
    }

    fn draw_o(&self, painter: &egui::Painter, rect: egui::Rect) {
        let padding = rect.width() * 0.2;
        let center = rect.center();
        let radius = (rect.width() / 2.0) - padding;
        let color = egui::Color32::from_rgb(50, 50, 220);
        let stroke = egui::Stroke::new(4.0, color);

        painter.circle_stroke(center, radius, stroke);
    }

    fn find_winning_line(state: &common::proto::tictactoe::TicTacToeGameState) -> Option<(u32, u32, u32, u32)> {
        let width = state.field_width as usize;
        let height = state.field_height as usize;
        let win_count = state.win_count as usize;

        let mut board = vec![vec![0i32; width]; height];
        for cell in &state.board {
            if cell.y < height as u32 && cell.x < width as u32 {
                board[cell.y as usize][cell.x as usize] = cell.mark;
            }
        }

        let winning_mark = if state.status == 2 { 2 } else if state.status == 3 { 3 } else { return None; };

        for y in 0..height {
            for x in 0..width {
                if board[y][x] == winning_mark {
                    if let Some(line) = Self::check_line_from(&board, x, y, 1, 0, win_count, winning_mark) {
                        return Some(line);
                    }
                    if let Some(line) = Self::check_line_from(&board, x, y, 0, 1, win_count, winning_mark) {
                        return Some(line);
                    }
                    if let Some(line) = Self::check_line_from(&board, x, y, 1, 1, win_count, winning_mark) {
                        return Some(line);
                    }
                    if let Some(line) = Self::check_line_from(&board, x, y, 1, -1, win_count, winning_mark) {
                        return Some(line);
                    }
                }
            }
        }

        None
    }

    fn check_line_from(
        board: &[Vec<i32>],
        start_x: usize,
        start_y: usize,
        dx: isize,
        dy: isize,
        win_count: usize,
        mark: i32,
    ) -> Option<(u32, u32, u32, u32)> {
        let height = board.len();
        let width = board[0].len();

        let mut count = 0;
        for i in 0..win_count {
            let x = start_x as isize + dx * i as isize;
            let y = start_y as isize + dy * i as isize;

            if x < 0 || y < 0 || x >= width as isize || y >= height as isize {
                return None;
            }

            if board[y as usize][x as usize] == mark {
                count += 1;
            } else {
                return None;
            }
        }

        if count == win_count {
            let end_x = start_x as isize + dx * (win_count as isize - 1);
            let end_y = start_y as isize + dy * (win_count as isize - 1);
            Some((start_x as u32, start_y as u32, end_x as u32, end_y as u32))
        } else {
            None
        }
    }

    fn render_info_panel(
        &self,
        ui: &mut egui::Ui,
        state: &common::proto::tictactoe::TicTacToeGameState,
        client_id: &str,
    ) {
        ui.heading("TicTacToe");
        ui.separator();

        if let (Some(player_x), Some(player_o)) = (&state.player_x, &state.player_o) {
            ui.label(format!("X: {}", if player_x.is_bot {
                format!("{} (Bot)", player_x.player_id)
            } else {
                player_x.player_id.clone()
            }));

            ui.label(format!("O: {}", if player_o.is_bot {
                format!("{} (Bot)", player_o.player_id)
            } else {
                player_o.player_id.clone()
            }));

            ui.separator();

            if let Some(current_player) = &state.current_player {
                let is_my_turn = current_player.player_id == client_id;
                let current_mark = if current_player.player_id == player_x.player_id { "X" } else { "O" };

                if is_my_turn {
                    ui.colored_label(egui::Color32::GREEN, format!("Your turn ({})", current_mark));
                } else {
                    ui.label(format!("{}'s turn ({})", current_player.player_id, current_mark));
                }
            }
        }

        ui.separator();

        match state.status {
            1 => ui.label("Game in progress"),
            2 => ui.colored_label(egui::Color32::GREEN, "X Won!"),
            3 => ui.colored_label(egui::Color32::GREEN, "O Won!"),
            4 => ui.label("Draw!"),
            _ => ui.label("Unknown status"),
        };
    }

    pub fn render_game_over(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        scores: &[ScoreEntry],
        winner: &Option<PlayerIdentity>,
        client_id: &str,
        last_game_state: &Option<GameStateUpdate>,
        _reason: &TicTacToeGameEndReason,
        play_again_status: &PlayAgainStatus,
        command_tx: &mpsc::UnboundedSender<ClientCommand>,
    ) {
        let state = if let Some(game_state_update) = last_game_state {
            match &game_state_update.state {
                Some(common::game_state_update::State::Tictactoe(ttt_state)) => ttt_state,
                _ => return,
            }
        } else {
            return;
        };

        let available_width = ui.available_width();
        let available_height = ui.available_height();

        let cell_size = Self::calculate_cell_size(
            available_width,
            available_height,
            state.field_width,
            state.field_height,
        );

        let board_width = cell_size * state.field_width as f32;
        let board_height = cell_size * state.field_height as f32;

        ui.heading("Game Over!");
        ui.separator();

        ui.horizontal(|ui| {
            let (rect, _response) = ui.allocate_exact_size(
                egui::vec2(board_width, board_height),
                egui::Sense::hover(),
            );

            let painter = ui.painter();

            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(240, 240, 240));

            for i in 0..=state.field_width {
                let x = rect.left() + i as f32 * cell_size;
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    egui::Stroke::new(Self::LINE_WIDTH, egui::Color32::BLACK),
                );
            }

            for i in 0..=state.field_height {
                let y = rect.top() + i as f32 * cell_size;
                painter.line_segment(
                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                    egui::Stroke::new(Self::LINE_WIDTH, egui::Color32::BLACK),
                );
            }

            for cell_mark in &state.board {
                let x = cell_mark.x;
                let y = cell_mark.y;

                let cell_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        rect.left() + x as f32 * cell_size,
                        rect.top() + y as f32 * cell_size,
                    ),
                    egui::vec2(cell_size, cell_size),
                );

                match cell_mark.mark {
                    2 => self.draw_x(painter, cell_rect),
                    3 => self.draw_o(painter, cell_rect),
                    _ => {}
                }
            }

            if let Some(winning_line) = Self::find_winning_line(state) {
                let (start_x, start_y, end_x, end_y) = winning_line;
                let start_pos = egui::pos2(
                    rect.left() + (start_x as f32 + 0.5) * cell_size,
                    rect.top() + (start_y as f32 + 0.5) * cell_size,
                );
                let end_pos = egui::pos2(
                    rect.left() + (end_x as f32 + 0.5) * cell_size,
                    rect.top() + (end_y as f32 + 0.5) * cell_size,
                );
                painter.line_segment(
                    [start_pos, end_pos],
                    egui::Stroke::new(6.0, egui::Color32::from_rgba_unmultiplied(50, 200, 50, 200)),
                );
            }

            ui.add_space(Self::BOARD_PADDING);

            ui.vertical(|ui| {
                ui.heading(egui::RichText::new("🏁 Game Over! 🏁").size(20.0));
                ui.separator();

                let won_mark = if let Some(winner) = winner {
                    if let Some(player_x) = &state.player_x {
                        if player_x.player_id == winner.player_id {
                            "X"
                        } else {
                            "O"
                        }
                    } else {
                        ""
                    }
                } else {
                    ""
                };

                if let Some(winner) = winner {
                    ui.label(egui::RichText::new(format!("Winner: {} ({})", winner.player_id, won_mark)).size(16.0).strong());
                    if winner.player_id == client_id {
                        ui.label(egui::RichText::new("🎉 Congratulations! You won! 🎉").size(14.0));
                    }
                } else {
                    ui.label(egui::RichText::new("It's a Draw!").size(16.0).strong());
                }

                ui.add_space(10.0);
                ui.separator();
                ui.heading("Final Scores:");

                let mut sorted_scores: Vec<_> = scores.iter().collect();
                sorted_scores.sort_by(|a, b| b.score.cmp(&a.score));

                for (rank, entry) in sorted_scores.iter().enumerate() {
                    let identity = entry.identity.as_ref().unwrap();
                    let egui_color = generate_color_from_client_id(&identity.player_id);

                    let is_bot = identity.is_bot;
                    let bot_marker = if is_bot { " [BOT]" } else { "" };

                    let is_you = !is_bot && identity.player_id == client_id;
                    let you_marker = if is_you { " (You)" } else { "" };
                    let medal = match rank {
                        0 => "🥇",
                        1 => "🥈",
                        _ => "  ",
                    };

                    ui.horizontal(|ui| {
                        ui.label(medal);
                        let text = format!("{}{}{} - {}", identity.player_id, bot_marker, you_marker, entry.score);
                        if is_you {
                            ui.colored_label(egui_color, format!("→ {}", text));
                        } else {
                            ui.label(text);
                        }
                    });
                }

                ui.add_space(10.0);
                ui.separator();

                match play_again_status {
                    PlayAgainStatus::Available { ready_players, pending_players } => {
                        let total = ready_players.len() + pending_players.len();
                        ui.label(format!("Play again? ({}/{})", ready_players.len(), total));

                        if ui.button("Play Again").clicked() {
                            let _ = command_tx.send(ClientCommand::Menu(MenuCommand::PlayAgain));
                        }
                    }
                    PlayAgainStatus::NotAvailable => {
                        ui.label("Play again not available");
                    }
                }

                if ui.button("Leave Game").clicked() {
                    let _ = command_tx.send(ClientCommand::Menu(MenuCommand::LeaveLobby));
                }
            });
        });
    }
}
