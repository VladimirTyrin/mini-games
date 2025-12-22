use eframe::egui;
use common::{GameStateUpdate, Snake, Position, Direction, ScoreEntry};
use tokio::sync::mpsc;
use crate::state::{GameCommand, MenuCommand, SharedState};
use crate::game_render::Sprites;

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
    sprite_textures_loaded: bool,
    last_input_direction: Option<Direction>,
    shared_state: SharedState,
}

impl GameUi {
    pub fn new(shared_state: SharedState) -> Self {
        Self {
            sprites: Sprites::load(),
            sprite_textures_loaded: false,
            last_input_direction: None,
            shared_state,
        }
    }

    pub fn render_game(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        session_id: &str,
        game_state: &Option<GameStateUpdate>,
        client_id: &str,
    ) {
        if let Some(state) = game_state {
            self.handle_input(ctx);

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
            let background_color = egui::Color32::from_rgb(0x88, 0xFF, 0x88);
            painter.rect_filled(rect, 0.0, background_color);

            for food in &state.food {
                self.render_sprite_at(
                    &painter,
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
                if !snake.alive {
                    continue;
                }

                let segments = &snake.segments;
                if segments.is_empty() {
                    continue;
                }

                let color = generate_color_from_client_id(&snake.client_id);

                for (i, segment) in segments.iter().enumerate() {
                    let sprite_name = format!("snake_{}_seg_{}", snake.client_id, i);

                    let sprite = if i == 0 {
                        let direction = if segments.len() > 1 {
                            Self::get_direction(&segments[0], &segments[1], field_width, field_height)
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
                        &painter,
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

            ui.separator();
            ui.heading("Scores:");
            for snake in &state.snakes {
                let is_you = snake.client_id == client_id;
                let status = if snake.alive { "🟢" } else { "💀" };
                let you_marker = if is_you { " (You)" } else { "" };
                ui.label(format!(
                    "{} {}{}: {} points",
                    status, snake.client_id, you_marker, snake.score
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
        scores: &[ScoreEntry],
        winner_id: &str,
        client_id: &str,
        menu_command_tx: &mpsc::UnboundedSender<MenuCommand>,
    ) {
        ui.heading("Game Over!");
        ui.separator();

        ui.label(format!("Winner: {}", winner_id));
        if winner_id == client_id {
            ui.label("🎉 Congratulations! You won! 🎉");
        }

        ui.separator();
        ui.heading("Final Scores:");

        let mut sorted_scores: Vec<_> = scores.iter().collect();
        sorted_scores.sort_by(|a, b| b.score.cmp(&a.score));

        for (rank, entry) in sorted_scores.iter().enumerate() {
            let is_you = entry.client_id == client_id;
            let you_marker = if is_you { " (You)" } else { "" };
            let medal = match rank {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "  ",
            };
            ui.label(format!(
                "{} {}. {}{}: {} points",
                medal,
                rank + 1,
                entry.client_id,
                you_marker,
                entry.score
            ));
        }

        ui.separator();
        if ui.button("Back to Lobby List").clicked() {
            self.shared_state.clear_game_command_tx();
            let _ = menu_command_tx.send(MenuCommand::ListLobbies);
        }
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
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
                    if let Some(game_command_tx) = self.shared_state.get_game_command_tx() {
                        let _ = game_command_tx.send(GameCommand::SendTurn { direction });
                        self.last_input_direction = Some(direction);
                    }
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
