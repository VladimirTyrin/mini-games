use common::Direction;
use eframe::egui;
use image::{ImageFormat, RgbaImage};

#[derive(Clone)]
pub struct Sprite {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
    name: String,
}

pub struct Sprites {
    head_left: Sprite,
    head_right: Sprite,
    head_up: Sprite,
    head_down: Sprite,

    tail_left: Sprite,
    tail_right: Sprite,
    tail_up: Sprite,
    tail_down: Sprite,

    body_horizontal: Sprite,
    body_vertical: Sprite,

    turn_ul: Sprite,
    turn_ur: Sprite,
    turn_dl: Sprite,
    turn_dr: Sprite,

    apple: Sprite,
}

impl Sprites {
    pub const PIXELS_PER_CELL: usize = 64;

    pub fn load() -> Self {
        let sprite_sheet = image::load_from_memory_with_format(
            include_bytes!("../assets/sprites.png"),
            ImageFormat::Png,
        )
        .expect("invalid sprite PNG")
        .to_rgba8();

        let head_right = Self::extract_sprite(&sprite_sheet, 4, 0, "head_right");
        let head_left = Self::extract_sprite(&sprite_sheet, 3, 1, "head_left");
        let head_down = Self::extract_sprite(&sprite_sheet, 4, 1, "head_down");
        let head_up = Self::extract_sprite(&sprite_sheet, 3, 0, "head_up");
        let apple = Self::extract_sprite(&sprite_sheet, 0, 3, "apple");

        let body_horizontal = Self::extract_sprite(&sprite_sheet, 1, 0, "body_horizontal");
        let body_vertical = Self::extract_sprite(&sprite_sheet, 2, 1, "body_vertical");

        let tail_left = Self::extract_sprite(&sprite_sheet, 3, 3, "tail_left");
        let tail_right = Self::extract_sprite(&sprite_sheet, 4, 2, "tail_right");
        let tail_down = Self::extract_sprite(&sprite_sheet, 4, 3, "tail_down");
        let tail_up = Self::extract_sprite(&sprite_sheet, 3, 2, "tail_up");

        let turn_ul = Self::extract_sprite(&sprite_sheet, 2, 0, "turn_ul");
        let turn_ur = Self::extract_sprite(&sprite_sheet, 0, 0, "turn_ur");
        let turn_dl = Self::extract_sprite(&sprite_sheet, 2, 2, "turn_dl");
        let turn_dr = Self::extract_sprite(&sprite_sheet, 0, 1, "turn_dr");

        Sprites {
            head_right,
            head_left,
            head_down,
            head_up,
            tail_left,
            tail_right,
            tail_down,
            tail_up,
            body_vertical,
            body_horizontal,
            turn_ul,
            turn_ur,
            turn_dl,
            turn_dr,
            apple,
        }
    }

    fn extract_sprite(sheet: &RgbaImage, col: u32, row: u32, name: &str) -> Sprite {
        let mut pixels = Vec::with_capacity(Self::PIXELS_PER_CELL * Self::PIXELS_PER_CELL * 4);

        let ppc: u32 = Self::PIXELS_PER_CELL as u32;
        for y in 0..ppc {
            for x in 0..ppc {
                let px = sheet.get_pixel(col * ppc + x, row * ppc + y);
                let [r, g, b, a] = px.0;
                pixels.push(r);
                pixels.push(g);
                pixels.push(b);
                pixels.push(a);
            }
        }

        Sprite {
            pixels,
            width: Self::PIXELS_PER_CELL,
            height: Self::PIXELS_PER_CELL,
            name: name.to_string(),
        }
    }

    pub fn get_head_sprite(&self, direction: Direction) -> &Sprite {
        match direction {
            Direction::Up => &self.head_up,
            Direction::Down => &self.head_down,
            Direction::Left => &self.head_left,
            Direction::Right => &self.head_right,
            _ => &self.head_up,
        }
    }

    pub fn get_tail_sprite(&self, from_x: i32, from_y: i32, to_x: i32, to_y: i32, field_width: u32, field_height: u32) -> &Sprite {
        let modulo_x = field_width as i32;
        let modulo_y = field_height as i32;

        if Self::less_modulo(from_y, to_y, modulo_y) {
            &self.tail_up
        } else if Self::greater_modulo(from_y, to_y, modulo_y) {
            &self.tail_down
        } else if Self::less_modulo(from_x, to_x, modulo_x) {
            &self.tail_left
        } else {
            &self.tail_right
        }
    }

    pub fn get_body_sprite(
        &self,
        prev_x: i32,
        prev_y: i32,
        curr_x: i32,
        curr_y: i32,
        next_x: i32,
        next_y: i32,
        field_width: u32,
        field_height: u32,
    ) -> &Sprite {
        let modulo_x = field_width as i32;
        let modulo_y = field_height as i32;

        let prev_left = Self::less_modulo(prev_x, curr_x, modulo_x);
        let prev_right = Self::greater_modulo(prev_x, curr_x, modulo_x);
        let prev_up = Self::less_modulo(prev_y, curr_y, modulo_y);
        let prev_down = Self::greater_modulo(prev_y, curr_y, modulo_y);

        let next_left = Self::less_modulo(next_x, curr_x, modulo_x);
        let next_right = Self::greater_modulo(next_x, curr_x, modulo_x);
        let next_up = Self::less_modulo(next_y, curr_y, modulo_y);
        let next_down = Self::greater_modulo(next_y, curr_y, modulo_y);

        if (prev_left && next_right) || (prev_right && next_left) {
            &self.body_horizontal
        } else if (prev_up && next_down) || (prev_down && next_up) {
            &self.body_vertical
        } else if (prev_up && next_left) || (prev_left && next_up) {
            &self.turn_dl
        } else if (prev_up && next_right) || (prev_right && next_up) {
            &self.turn_dr
        } else if (prev_down && next_left) || (prev_left && next_down) {
            &self.turn_ul
        } else {
            &self.turn_ur
        }
    }

    pub fn get_apple_sprite(&self) -> &Sprite {
        &self.apple
    }

    fn greater_modulo(a: i32, b: i32, modulo: i32) -> bool {
        if a == (b + 1) % modulo {
            return true;
        }
        false
    }

    fn less_modulo(a: i32, b: i32, modulo: i32) -> bool {
        if b == (a + 1) % modulo {
            return true;
        }
        false
    }
}

impl Sprite {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn to_egui_texture(
        &self,
        ctx: &egui::Context,
        name: &str,
    ) -> egui::TextureHandle {
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [self.width, self.height],
            &self.pixels,
        );
        ctx.load_texture(name, color_image, Default::default())
    }
}
