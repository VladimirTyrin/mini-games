use crate::game_render::Sprites;
use crate::state::{GameCommand, MenuCommand, ClientCommand, PlayAgainStatus};
use common::{Direction, GameStateUpdate, Position, ScoreEntry, PlayerIdentity};
use eframe::egui;
use tokio::sync::mpsc;

#[derive(Clone, Copy, Debug)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

fn generate_color_from_client_id(client_id: &str) -> Color {
    let hash = client_id.bytes().fold(0u32, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as u32)
    });

    let hue = (hash % 360) as f32;
    let saturation = 0.7_f32;
    let lightness = 0.5_f32;

    let c = (1.0_f32 - (2.0_f32 * lightness - 1.0_f32).abs()) * saturation;
    let x = c * (1.0_f32 - ((hue / 60.0_f32) % 2.0_f32 - 1.0_f32).abs());
    let m = lightness - c / 2.0;

    let (r, g, b) = if hue < 60.0 {
        (c, x, 0.0)
    } else if hue < 120.0 {
        (x, c, 0.0)
    } else if hue < 180.0 {
        (0.0, c, x)
    } else if hue < 240.0 {
        (0.0, x, c)
    } else if hue < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Color {
        r: ((r + m) * 255.0) as u8,
        g: ((g + m) * 255.0) as u8,
        b: ((b + m) * 255.0) as u8,
    }
}

pub struct GameUi {
    sprites: Sprites,
    last_input_direction: Option<Direction>,
}

impl GameUi {
    pub fn new() -> Self {
        Self {
            sprites: Sprites::load(),
            last_input_direction: None,
        }
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

            let field_width = state.field_width;
            let field_height = state.field_height;

            let pixels_per_cell = Sprites::PIXELS_PER_CELL as f32;
            let canvas_width = field_width as f32 * pixels_per_cell;
            let canvas_height = field_height as f32 * pixels_per_cell;

            ui.heading(format!("Game Session: {}", session_id));
            ui.separator();

            let (response, painter) =
                ui.allocate_painter(egui::Vec2::new(canvas_width, canvas_height), egui::Sense::hover());

            let rect = response.rect;
            self.render_game_field(&painter, ctx, rect, state, false);

            ui.separator();
            ui.heading("Scores:");
            for snake in &state.snakes {
                let player_id = snake.identity.as_ref()
                    .map(|i| i.player_id.clone())
                    .unwrap_or_else(|| "Unknown".to_string());

                let is_bot = snake.identity.as_ref().map(|i| i.is_bot).unwrap_or(false);
                let bot_marker = if is_bot { " [BOT]" } else { "" };

                let is_you = !is_bot && player_id == client_id;
                let status = if snake.alive { "🟢" } else { "💀" };
                let you_marker = if is_you { " (You)" } else { "" };
                ui.label(format!(
                    "{} {}{}{}: {} points",
                    status, player_id, bot_marker, you_marker, snake.score
                ));
            }
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
            let field_width = state.field_width;
            let field_height = state.field_height;

            let pixels_per_cell = Sprites::PIXELS_PER_CELL as f32;
            let canvas_width = field_width as f32 * pixels_per_cell;
            let canvas_height = field_height as f32 * pixels_per_cell;

            ui.heading("Game Over!");
            ui.separator();

            let (response, painter) =
                ui.allocate_painter(egui::Vec2::new(canvas_width, canvas_height), egui::Sense::hover());

            let rect = response.rect;
            self.render_game_field(&painter, ctx, rect, state, true);

            let overlay_color = egui::Color32::from_black_alpha(100);
            painter.rect_filled(rect, 0.0, overlay_color);

            let center = rect.center();
            let overlay_width = canvas_width * 0.8;
            let overlay_height = canvas_height * 0.6;
            let overlay_rect = egui::Rect::from_center_size(
                center,
                egui::vec2(overlay_width, overlay_height),
            );

            painter.rect_filled(
                overlay_rect,
                8.0,
                egui::Color32::from_rgba_premultiplied(40, 40, 40, 200),
            );
            painter.rect_stroke(
                overlay_rect,
                8.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(200, 200, 200)),
                egui::epaint::StrokeKind::Outside,
            );

            ui.scope_builder(egui::UiBuilder::new().max_rect(overlay_rect.shrink(20.0)), |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.heading(egui::RichText::new("🏁 Game Over! 🏁").size(24.0).color(egui::Color32::WHITE));
                    ui.add_space(10.0);

                    let winner_name = winner.as_ref()
                        .map(|w| w.player_id.clone())
                        .unwrap_or_else(|| "None".to_string());
                    ui.label(egui::RichText::new(format!("Winner: {}", winner_name)).size(18.0).color(egui::Color32::from_rgb(255, 215, 0)));
                    if winner_name == client_id {
                        ui.label(egui::RichText::new("🎉 Congratulations! You won! 🎉").size(16.0).color(egui::Color32::from_rgb(255, 215, 0)));
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
                    ui.label(egui::RichText::new(reason_text).size(14.0).color(egui::Color32::from_rgb(200, 200, 200)));

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(5.0);

                    ui.label(egui::RichText::new("Final Scores:").size(16.0).color(egui::Color32::WHITE));
                    ui.add_space(5.0);

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
                        let text_color = if is_winner {
                            egui::Color32::from_rgb(255, 215, 0)
                        } else if is_you {
                            egui::Color32::from_rgb(255, 255, 100)
                        } else {
                            egui::Color32::WHITE
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{}. {}{}{}: {} points",
                                rank + 1,
                                player_id,
                                bot_marker,
                                you_marker,
                                entry.score
                            ))
                            .size(14.0)
                            .color(text_color)
                        );
                    }

                    ui.add_space(10.0);

                    match play_again_status {
                        PlayAgainStatus::Available { ready_players, pending_players } => {
                            if pending_players.is_empty() {
                                ui.label(egui::RichText::new("Starting new game...").size(14.0).color(egui::Color32::from_rgb(100, 255, 100)));
                            } else {
                                let is_ready = ready_players.iter().any(|p| p.player_id == client_id);
                                if is_ready {
                                    ui.label(egui::RichText::new("Waiting for other players...").size(14.0).color(egui::Color32::from_rgb(255, 215, 0)));
                                } else {
                                    if ui.button(egui::RichText::new("Play Again (R)").size(16.0)).clicked() {
                                        let _ = command_tx.send(ClientCommand::Menu(MenuCommand::PlayAgain));
                                    }
                                    ctx.input(|i| {
                                        if i.key_pressed(egui::Key::R) {
                                            let _ = command_tx.send(ClientCommand::Menu(MenuCommand::PlayAgain));
                                        }
                                    });
                                    ctx.request_repaint();
                                }

                                ui.add_space(5.0);
                                ui.label(egui::RichText::new("Players ready:").size(12.0).color(egui::Color32::from_rgb(200, 200, 200)));
                                for ready_player in ready_players {
                                    let is_you = ready_player.player_id == client_id;
                                    let you_marker = if is_you { " (You)" } else { "" };
                                    ui.label(egui::RichText::new(format!("✓ {}{}", ready_player.player_id, you_marker)).size(12.0).color(egui::Color32::from_rgb(100, 255, 100)));
                                }
                                if !pending_players.is_empty() {
                                    ui.label(egui::RichText::new("Waiting for:").size(12.0).color(egui::Color32::from_rgb(200, 200, 200)));
                                    for pending_player in pending_players {
                                        let is_you = pending_player.player_id == client_id;
                                        let you_marker = if is_you { " (You)" } else { "" };
                                        ui.label(egui::RichText::new(format!("⏳ {}{}", pending_player.player_id, you_marker)).size(12.0).color(egui::Color32::from_rgb(255, 215, 0)));
                                    }
                                }
                                ui.add_space(5.0);
                            }
                        }
                        PlayAgainStatus::NotAvailable => {
                            ui.label(egui::RichText::new("Play again not available").size(12.0).color(egui::Color32::from_rgb(150, 150, 150)));
                            ui.label(egui::RichText::new("(A player left the lobby)").size(10.0).color(egui::Color32::from_rgb(150, 150, 150)));
                            ui.add_space(5.0);
                        }
                    }

                    if ui.button(egui::RichText::new("Back to Lobby List (Esc)").size(14.0)).clicked() {
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
                ui.label(format!(
                    "{} {}. {}{}{}: {} points",
                    medal,
                    rank + 1,
                    player_id,
                    bot_marker,
                    you_marker,
                    entry.score
                ));
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
                if Some(direction) != self.last_input_direction {
                    let _ = command_tx.send(ClientCommand::Game(GameCommand::SendTurn { direction }));
                    self.last_input_direction = Some(direction);
                }
            }
        });

        ctx.request_repaint();
    }

    fn render_sprite_at(
        &self,
        painter: &egui::Painter,
        ctx: &egui::Context,
        sprite: &crate::game_render::Sprite,
        grid_x: i32,
        grid_y: i32,
        canvas_min: egui::Pos2,
        pixels_per_cell: f32,
        sprite_name: &str,
        tint: Color,
    ) {
        let texture = sprite.to_egui_texture(ctx, sprite_name);

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
        show_dead_snakes: bool,
    ) {
        let field_width = state.field_width;
        let field_height = state.field_height;
        let pixels_per_cell = Sprites::PIXELS_PER_CELL as f32;

        let background_color = egui::Color32::from_rgb(0x88, 0xFF, 0x88);
        painter.rect_filled(rect, 0.0, background_color);

        for food in &state.food {
            self.render_sprite_at(
                painter,
                ctx,
                &self.sprites.get_apple_sprite(),
                food.x,
                food.y,
                rect.min,
                pixels_per_cell,
                "apple",
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
                generate_color_from_client_id(&player_id)
            } else {
                Color { r: 128, g: 128, b: 128 }
            };

            for (i, segment) in segments.iter().enumerate() {
                let sprite_name = if show_dead_snakes {
                    format!("snake_{}_seg_{}_final", player_id, i)
                } else {
                    format!("snake_{}_seg_{}", player_id, i)
                };

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

                self.render_sprite_at(
                    painter,
                    ctx,
                    sprite,
                    segment.x,
                    segment.y,
                    rect.min,
                    pixels_per_cell,
                    &sprite_name,
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
