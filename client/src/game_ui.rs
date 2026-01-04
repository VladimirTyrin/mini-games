use crate::sprites::Sprites;
use crate::state::{GameCommand, MenuCommand, ClientCommand, PlayAgainStatus};
use crate::colors::generate_color_from_client_id;
use common::{Direction, GameStateUpdate, Position, ScoreEntry, PlayerIdentity};
use eframe::egui;
use tokio::sync::mpsc;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

pub struct GameUi {
    sprites: Sprites,
    texture_cache: HashMap<String, egui::TextureHandle>,
    cached_cell_size: f32,
}

impl GameUi {
    const SCORES_AREA_WIDTH: f32 = 150.0;
    const MIN_CELL_SIZE: f32 = 16.0;
    const MAX_CELL_SIZE: f32 = 128.0;
    const PADDING: f32 = 20.0;

    pub fn new() -> Self {
        Self {
            sprites: Sprites::load(),
            texture_cache: HashMap::new(),
            cached_cell_size: 64.0,
        }
    }

    fn check_and_invalidate_cache(&mut self, new_cell_size: f32) {
        const RESIZE_THRESHOLD: f32 = 1.0;

        if (new_cell_size - self.cached_cell_size).abs() > RESIZE_THRESHOLD {
            self.texture_cache.clear();
            self.cached_cell_size = new_cell_size;
        }
    }

    fn get_or_create_texture(
        &mut self,
        ctx: &egui::Context,
        sprite: crate::sprites::Sprite,
        cache_key: String,
    ) -> egui::TextureHandle {
        if let Some(texture) = self.texture_cache.get(&cache_key) {
            return texture.clone();
        }

        let texture = sprite.to_egui_texture(ctx, &cache_key);
        self.texture_cache.insert(cache_key.clone(), texture.clone());
        texture
    }

    fn calculate_cell_size(
        available_width: f32,
        available_height: f32,
        field_width: u32,
        field_height: u32,
    ) -> f32 {
        let available_field_width = available_width - Self::SCORES_AREA_WIDTH - (Self::PADDING * 2.0);
        let available_field_height = available_height - (Self::PADDING * 2.0);

        let cell_width = available_field_width / field_width as f32;
        let cell_height = available_field_height / field_height as f32;

        let cell_size = cell_width.min(cell_height);

        cell_size.clamp(Self::MIN_CELL_SIZE, Self::MAX_CELL_SIZE)
    }

    pub fn render_game(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        session_id: &str,
        game_state: &Option<GameStateUpdate>,
        client_id: &str,
        command_tx: &mpsc::UnboundedSender<ClientCommand>,
    ) {
        if let Some(state) = game_state {
            self.handle_input(ctx, command_tx);

            let available_width = ui.available_width();
            let available_height = ui.available_height();

            let pixels_per_cell = Self::calculate_cell_size(
                available_width,
                available_height,
                state.field_width,
                state.field_height,
            );

            self.check_and_invalidate_cache(pixels_per_cell);

            let canvas_width = state.field_width as f32 * pixels_per_cell;
            let canvas_height = state.field_height as f32 * pixels_per_cell;

            ui.heading(format!("Game Session: {}", session_id));
            ui.separator();

            ui.horizontal(|ui| {
                let (response, painter) =
                    ui.allocate_painter(egui::Vec2::new(canvas_width, canvas_height), egui::Sense::hover());

                let rect = response.rect;
                let show_dead_snakes = matches!(
                    common::DeadSnakeBehavior::try_from(state.dead_snake_behavior),
                    Ok(common::DeadSnakeBehavior::StayOnField)
                );
                self.render_game_field(&painter, ctx, rect, state, pixels_per_cell, show_dead_snakes);

                ui.add_space(Self::PADDING);

                ui.vertical(|ui| {
                    ui.heading("Scores:");
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .max_height(canvas_height)
                        .show(ui, |ui| {
                            for snake in &state.snakes {
                                let player_id = snake.identity.as_ref()
                                    .map(|i| i.player_id.clone())
                                    .unwrap_or_else(|| "Unknown".to_string());

                                let is_bot = snake.identity.as_ref().map(|i| i.is_bot).unwrap_or(false);
                                let bot_marker = if is_bot { " [BOT]" } else { "" };

                                let is_you = !is_bot && player_id == client_id;
                                let status = if snake.alive { "🟢" } else { "💀" };
                                let you_marker = if is_you { " (You)" } else { "" };

                                ui.horizontal(|ui| {
                                    let color = generate_color_from_client_id(&player_id);
                                    let head_sprite = self.sprites.get_head_sprite(Direction::Right).clone();
                                    let cache_key = format!("game_score_head_{}", player_id);
                                    let texture = self.get_or_create_texture(ctx, head_sprite, cache_key);

                                    let icon_size = (pixels_per_cell * 0.3).clamp(16.0, 32.0);
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

                                    ui.label(format!(
                                        "{} {}{}{}: {} points",
                                        status, player_id, bot_marker, you_marker, snake.score
                                    ));
                                });
                            }
                        });
                });
            });
        } else {
            ui.heading("Waiting for game to start...");
            ui.spinner();
        }
    }

    pub fn render_game_over(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        scores: &[ScoreEntry],
        winner: &Option<PlayerIdentity>,
        client_id: &str,
        last_game_state: &Option<GameStateUpdate>,
        reason: &common::GameEndReason,
        play_again_status: &PlayAgainStatus,
        command_tx: &mpsc::UnboundedSender<ClientCommand>,
    ) {
        if let Some(state) = last_game_state {
            let available_width = ui.available_width();
            let available_height = ui.available_height();

            let pixels_per_cell = Self::calculate_cell_size(
                available_width,
                available_height,
                state.field_width,
                state.field_height,
            );

            self.check_and_invalidate_cache(pixels_per_cell);

            let canvas_width = state.field_width as f32 * pixels_per_cell;
            let canvas_height = state.field_height as f32 * pixels_per_cell;

            ui.heading("Game Over!");
            ui.separator();

            ui.horizontal(|ui| {
                let (response, painter) =
                    ui.allocate_painter(egui::Vec2::new(canvas_width, canvas_height), egui::Sense::hover());

                let rect = response.rect;
                self.render_game_field(&painter, ctx, rect, state, pixels_per_cell, true);

                ui.add_space(Self::PADDING);

                ui.vertical(|ui| {
                    ui.heading(egui::RichText::new("🏁 Game Over! 🏁").size(20.0));
                    ui.separator();

                    let winner_name = winner.as_ref()
                        .map(|w| w.player_id.clone())
                        .unwrap_or_else(|| "None".to_string());
                    ui.label(egui::RichText::new(format!("Winner: {}", winner_name)).size(16.0).strong());
                    if winner_name == client_id {
                        ui.label(egui::RichText::new("🎉 Congratulations! You won! 🎉").size(14.0));
                    }

                    ui.add_space(5.0);
                    let reason_text = match reason {
                        common::GameEndReason::WallCollision => "💥 Game ended: Wall collision",
                        common::GameEndReason::SelfCollision => "🐍 Game ended: Self collision",
                        common::GameEndReason::SnakeCollision => "💥 Game ended: Snake collision",
                        common::GameEndReason::PlayerDisconnected => "📡 Game ended: Player disconnected",
                        common::GameEndReason::GameCompleted => "✅ Game completed",
                        _ => "Game ended",
                    };
                    ui.label(reason_text);

                    ui.add_space(10.0);
                    ui.separator();
                    ui.heading("Final Scores:");

                    egui::ScrollArea::vertical()
                        .max_height(canvas_height - 300.0)
                        .show(ui, |ui| {
                            let mut sorted_scores: Vec<_> = scores.iter().collect();
                            sorted_scores.sort_by(|a, b| b.score.cmp(&a.score));

                            for (rank, entry) in sorted_scores.iter().enumerate() {
                                let player_id = entry.identity.as_ref()
                                    .map(|i| i.player_id.clone())
                                    .unwrap_or_else(|| "Unknown".to_string());

                                let is_bot = entry.identity.as_ref().map(|i| i.is_bot).unwrap_or(false);
                                let bot_marker = if is_bot { " [BOT]" } else { "" };

                                let is_you = !is_bot && player_id == client_id;
                                let is_winner = winner.as_ref().map(|w| w.player_id == player_id).unwrap_or(false);
                                let you_marker = if is_you { " (You)" } else { "" };
                                let medal = match rank {
                                    0 => "🥇",
                                    1 => "🥈",
                                    2 => "🥉",
                                    _ => "  ",
                                };

                                ui.horizontal(|ui| {
                                    let color = generate_color_from_client_id(&player_id);
                                    let head_sprite = self.sprites.get_head_sprite(Direction::Right).clone();
                                    let cache_key = format!("gameover_score_head_{}", player_id);
                                    let texture = self.get_or_create_texture(ctx, head_sprite, cache_key);

                                    let icon_size = (pixels_per_cell * 0.3).clamp(16.0, 32.0);
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

                                    let mut text = egui::RichText::new(format!(
                                        "{} {}. {}{}{}: {} points",
                                        medal,
                                        rank + 1,
                                        player_id,
                                        bot_marker,
                                        you_marker,
                                        entry.score
                                    ));

                                    if is_winner || is_you {
                                        text = text.strong();
                                    }

                                    ui.label(text);
                                });
                            }
                        });

                    ui.add_space(10.0);

                    match play_again_status {
                        PlayAgainStatus::Available { ready_players, pending_players } => {
                            if pending_players.is_empty() {
                                ui.label("Starting new game...");
                            } else {
                                let is_ready = ready_players.iter().any(|p| p.player_id == client_id);
                                if is_ready {
                                    ui.label("Waiting for other players...");
                                } else {
                                    if ui.button("Play Again (R)").clicked() {
                                        let _ = command_tx.send(ClientCommand::Menu(MenuCommand::PlayAgain));
                                    }
                                    ctx.input(|i| {
                                        if i.key_pressed(egui::Key::R) {
                                            let _ = command_tx.send(ClientCommand::Menu(MenuCommand::PlayAgain));
                                        }
                                    });
                                }

                                ui.add_space(5.0);
                                ui.label("Players ready:");
                                for ready_player in ready_players {
                                    let is_you = ready_player.player_id == client_id;
                                    let you_marker = if is_you { " (You)" } else { "" };
                                    ui.label(format!("✓ {}{}", ready_player.player_id, you_marker));
                                }
                                if !pending_players.is_empty() {
                                    ui.label("Waiting for:");
                                    for pending_player in pending_players {
                                        let is_you = pending_player.player_id == client_id;
                                        let you_marker = if is_you { " (You)" } else { "" };
                                        ui.label(format!("⏳ {}{}", pending_player.player_id, you_marker));
                                    }
                                }
                                ui.add_space(5.0);
                            }
                        }
                        PlayAgainStatus::NotAvailable => {
                            ui.label("Play again not available");
                            ui.label("(A player left the lobby)");
                            ui.add_space(5.0);
                        }
                    }

                    if ui.button("Back to Lobby List (Esc)").clicked() {
                        let _ = command_tx.send(ClientCommand::Menu(MenuCommand::LeaveLobby));
                    }

                    ctx.input(|i| {
                        if i.key_pressed(egui::Key::Escape) {
                            let _ = command_tx.send(ClientCommand::Menu(MenuCommand::LeaveLobby));
                        }
                    });
                });
            });
        } else {
            ui.heading("Game Over!");
            ui.separator();

            let winner_name = winner.as_ref()
                .map(|w| w.player_id.clone())
                .unwrap_or_else(|| "None".to_string());
            ui.label(format!("Winner: {}", winner_name));
            if winner_name == client_id {
                ui.label("🎉 Congratulations! You won! 🎉");
            }

            ui.separator();
            ui.heading("Final Scores:");

            let mut sorted_scores: Vec<_> = scores.iter().collect();
            sorted_scores.sort_by(|a, b| b.score.cmp(&a.score));

            for (rank, entry) in sorted_scores.iter().enumerate() {
                let player_id = entry.identity.as_ref()
                    .map(|i| i.player_id.clone())
                    .unwrap_or_else(|| "Unknown".to_string());

                let is_bot = entry.identity.as_ref().map(|i| i.is_bot).unwrap_or(false);
                let bot_marker = if is_bot { " [BOT]" } else { "" };

                let is_you = !is_bot && player_id == client_id;
                let you_marker = if is_you { " (You)" } else { "" };
                let medal = match rank {
                    0 => "🥇",
                    1 => "🥈",
                    2 => "🥉",
                    _ => "  ",
                };

                ui.horizontal(|ui| {
                    let color = generate_color_from_client_id(&player_id);
                    let head_sprite = self.sprites.get_head_sprite(Direction::Right);
                    let texture = head_sprite.to_egui_texture(ctx, &format!("score_fallback_head_{}", player_id));

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

                    ui.label(format!(
                        "{} {}. {}{}{}: {} points",
                        medal,
                        rank + 1,
                        player_id,
                        bot_marker,
                        you_marker,
                        entry.score
                    ));
                });
            }

            ui.separator();

            match play_again_status {
                PlayAgainStatus::Available { ready_players, pending_players } => {
                    if pending_players.is_empty() {
                        ui.label("Starting new game...");
                    } else {
                        let is_ready = ready_players.iter().any(|p| p.player_id == client_id);
                        if is_ready {
                            ui.label("Waiting for other players...");
                        } else if ui.button("Play Again").clicked() {
                            let _ = command_tx.send(ClientCommand::Menu(MenuCommand::PlayAgain));
                        }

                        ui.label("Players ready:");
                        for ready_player in ready_players {
                            let is_you = ready_player.player_id == client_id;
                            let you_marker = if is_you { " (You)" } else { "" };
                            ui.label(format!("✓ {}{}", ready_player.player_id, you_marker));
                        }
                        if !pending_players.is_empty() {
                            ui.label("Waiting for:");
                            for pending_player in pending_players {
                                let is_you = pending_player.player_id == client_id;
                                let you_marker = if is_you { " (You)" } else { "" };
                                ui.label(format!("⏳ {}{}", pending_player.player_id, you_marker));
                            }
                        }
                    }
                }
                PlayAgainStatus::NotAvailable => {
                    ui.label("Play again not available (A player left the lobby)");
                }
            }

            ui.separator();
            if ui.button("Back to Lobby List (Esc)").clicked() {
                let _ = command_tx.send(ClientCommand::Menu(MenuCommand::LeaveLobby));
            }

            ctx.input(|i| {
                if i.key_pressed(egui::Key::Escape) {
                    let _ = command_tx.send(ClientCommand::Menu(MenuCommand::LeaveLobby));
                }
            });
        }
    }

    fn handle_input(&mut self, ctx: &egui::Context, command_tx: &mpsc::UnboundedSender<ClientCommand>) {
        ctx.input(|i| {
            let mut new_direction = None;

            if i.key_pressed(egui::Key::ArrowUp) {
                new_direction = Some(Direction::Up);
            } else if i.key_pressed(egui::Key::ArrowDown) {
                new_direction = Some(Direction::Down);
            } else if i.key_pressed(egui::Key::ArrowLeft) {
                new_direction = Some(Direction::Left);
            } else if i.key_pressed(egui::Key::ArrowRight) {
                new_direction = Some(Direction::Right);
            }

            if let Some(direction) = new_direction {
                let _ = command_tx.send(ClientCommand::Game(GameCommand::SendTurn { direction }));
            }
        });
    }

    fn render_sprite_at(
        &mut self,
        painter: &egui::Painter,
        ctx: &egui::Context,
        sprite: crate::sprites::Sprite,
        grid_x: i32,
        grid_y: i32,
        canvas_min: egui::Pos2,
        pixels_per_cell: f32,
        cache_key: String,
        tint: Color,
    ) {
        let texture = self.get_or_create_texture(ctx, sprite, cache_key);

        let pos_x = canvas_min.x + grid_x as f32 * pixels_per_cell;
        let pos_y = canvas_min.y + grid_y as f32 * pixels_per_cell;

        let rect = egui::Rect::from_min_size(
            egui::pos2(pos_x, pos_y),
            egui::vec2(pixels_per_cell, pixels_per_cell),
        );

        painter.image(
            texture.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::from_rgb(tint.r, tint.g, tint.b),
        );
    }

    fn render_game_field(
        &mut self,
        painter: &egui::Painter,
        ctx: &egui::Context,
        rect: egui::Rect,
        state: &GameStateUpdate,
        pixels_per_cell: f32,
        show_dead_snakes: bool,
    ) {
        let field_width = state.field_width;
        let field_height = state.field_height;

        let background_color = egui::Color32::from_rgb(0x88, 0xFF, 0x88);
        painter.rect_filled(rect, 0.0, background_color);

        for food in &state.food {
            self.render_sprite_at(
                painter,
                ctx,
                self.sprites.get_apple_sprite().clone(),
                food.x,
                food.y,
                rect.min,
                pixels_per_cell,
                format!("apple_{}_{}", food.x, food.y),
                Color { r: 255, g: 255, b: 255 },
            );
        }

        for snake in &state.snakes {
            if !show_dead_snakes && !snake.alive {
                continue;
            }

            let segments = &snake.segments;
            if segments.is_empty() {
                continue;
            }

            let player_id = snake.identity.as_ref()
                .map(|i| i.player_id.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            let color = if snake.alive {
                let egui_color = generate_color_from_client_id(&player_id);
                Color {
                    r: egui_color.r(),
                    g: egui_color.g(),
                    b: egui_color.b(),
                }
            } else {
                Color { r: 128, g: 128, b: 128 }
            };

            for (i, segment) in segments.iter().enumerate() {
                let sprite = if i == 0 {
                    let direction = if segments.len() > 1 {
                        Self::get_direction(&segments[1], &segments[0], field_width, field_height)
                    } else {
                        Direction::Up
                    };
                    self.sprites.get_head_sprite(direction)
                } else if i == segments.len() - 1 {
                    let prev = &segments[i - 1];
                    self.sprites.get_tail_sprite(
                        prev.x,
                        prev.y,
                        segment.x,
                        segment.y,
                        field_width,
                        field_height,
                    )
                } else {
                    let prev = &segments[i - 1];
                    let next = &segments[i + 1];
                    self.sprites.get_body_sprite(
                        prev.x,
                        prev.y,
                        segment.x,
                        segment.y,
                        next.x,
                        next.y,
                        field_width,
                        field_height,
                    )
                };

                let cache_key = if show_dead_snakes {
                    format!("{}_{}_final", player_id, sprite.name())
                } else {
                    format!("{}_{}", player_id, sprite.name())
                };

                self.render_sprite_at(
                    painter,
                    ctx,
                    sprite.clone(),
                    segment.x,
                    segment.y,
                    rect.min,
                    pixels_per_cell,
                    cache_key,
                    color,
                );
            }
        }
    }

    fn get_direction(from: &Position, to: &Position, field_width: u32, field_height: u32) -> Direction {
        let dx = (to.x - from.x + field_width as i32) % field_width as i32;
        let dy = (to.y - from.y + field_height as i32) % field_height as i32;

        if dx == 1 || dx == -(field_width as i32 - 1) {
            Direction::Right
        } else if dx == field_width as i32 - 1 || dx == -1 {
            Direction::Left
        } else if dy == 1 || dy == -(field_height as i32 - 1) {
            Direction::Down
        } else {
            Direction::Up
        }
    }
}
