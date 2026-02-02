use std::collections::HashMap;

use super::box_entity::BoxEntity;
use super::types::Point;

pub struct BoxLandedInfo {
    pub box_id: u32,
    pub x: i32,
    pub y: i32,
}

pub struct Field {
    width: u32,
    height: u32,
    boxes: HashMap<u32, BoxEntity>,
    grid: Vec<Vec<Option<u32>>>,
}

impl Field {
    pub fn new(width: u32, height: u32) -> Self {
        let grid = vec![vec![None; width as usize]; height as usize];
        Self {
            width,
            height,
            boxes: HashMap::new(),
            grid,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn is_valid_x(&self, x: i32) -> bool {
        x >= 0 && x < self.width as i32
    }

    pub fn is_valid_position(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32
    }

    pub fn is_occupied(&self, x: i32, y: i32) -> bool {
        if !self.is_valid_position(x, y) {
            return false;
        }
        self.grid[y as usize][x as usize].is_some()
    }

    pub fn get_box_id_at(&self, x: i32, y: i32) -> Option<u32> {
        if !self.is_valid_position(x, y) {
            return None;
        }
        self.grid[y as usize][x as usize]
    }

    pub fn add_box(&mut self, box_entity: BoxEntity) {
        let pos = box_entity.position;
        let id = box_entity.id;

        if self.is_valid_position(pos.x, pos.y) {
            self.grid[pos.y as usize][pos.x as usize] = Some(id);
        }
        self.boxes.insert(id, box_entity);
    }

    pub fn remove_box(&mut self, id: u32) -> Option<BoxEntity> {
        if let Some(box_entity) = self.boxes.remove(&id) {
            let pos = box_entity.position;
            if self.is_valid_position(pos.x, pos.y) {
                self.grid[pos.y as usize][pos.x as usize] = None;
            }
            return Some(box_entity);
        }
        None
    }

    pub fn move_box(&mut self, id: u32, new_x: i32) {
        let old_pos = match self.boxes.get(&id) {
            Some(b) => b.position,
            None => return,
        };

        if self.is_valid_position(old_pos.x, old_pos.y) {
            self.grid[old_pos.y as usize][old_pos.x as usize] = None;
        }
        if self.is_valid_position(new_x, old_pos.y) {
            self.grid[old_pos.y as usize][new_x as usize] = Some(id);
        }

        if let Some(box_entity) = self.boxes.get_mut(&id) {
            box_entity.position.x = new_x;
        }

        self.make_column_fall(old_pos.x, old_pos.y + 1);
    }

    fn make_column_fall(&mut self, x: i32, start_y: i32) {
        for y in start_y..self.height as i32 {
            if let Some(box_id) = self.get_box_id_at(x, y) {
                let below_y = y - 1;
                let support_box_id = if below_y >= 0 {
                    self.get_box_id_at(x, below_y)
                } else {
                    None
                };

                let has_solid_support = if below_y < 0 {
                    true
                } else if let Some(support_id) = support_box_id {
                    self.boxes.get(&support_id).map(|b| !b.falling).unwrap_or(false)
                } else {
                    false
                };

                if !has_solid_support {
                    if let Some(b) = self.boxes.get_mut(&box_id) {
                        b.falling = true;
                    }
                }
            }
        }
    }

    pub fn update_falling_boxes(&mut self) -> Vec<BoxLandedInfo> {
        let mut landed = Vec::new();
        let mut box_ids: Vec<u32> = self.boxes.keys().copied().collect();
        box_ids.sort();

        for id in box_ids {
            let (pos, is_falling) = match self.boxes.get(&id) {
                Some(b) => (b.position, b.falling),
                None => continue,
            };

            if !is_falling {
                continue;
            }

            let below_y = pos.y - 1;
            let should_land = below_y < 0 || self.is_occupied(pos.x, below_y);

            if should_land {
                if let Some(b) = self.boxes.get_mut(&id) {
                    b.falling = false;
                }
                landed.push(BoxLandedInfo {
                    box_id: id,
                    x: pos.x,
                    y: pos.y,
                });
            } else {
                if self.is_valid_position(pos.x, pos.y) {
                    self.grid[pos.y as usize][pos.x as usize] = None;
                }
                if self.is_valid_position(pos.x, below_y) {
                    self.grid[below_y as usize][pos.x as usize] = Some(id);
                }
                if let Some(b) = self.boxes.get_mut(&id) {
                    b.position.y = below_y;
                }
            }
        }

        landed
    }

    pub fn check_and_clear_lines(&mut self) -> Vec<i32> {
        let mut cleared_lines = Vec::new();

        for y in 0..self.height as i32 {
            let is_full = (0..self.width as i32).all(|x| self.is_occupied(x, y));
            if is_full {
                cleared_lines.push(y);
            }
        }

        cleared_lines.sort_by(|a, b| b.cmp(a));

        for &y in &cleared_lines {
            for x in 0..self.width as i32 {
                if let Some(id) = self.grid[y as usize][x as usize] {
                    self.boxes.remove(&id);
                }
                self.grid[y as usize][x as usize] = None;
            }

            for above_y in (y + 1)..self.height as i32 {
                for x in 0..self.width as i32 {
                    if let Some(id) = self.grid[above_y as usize][x as usize] {
                        self.grid[above_y as usize][x as usize] = None;
                        self.grid[(above_y - 1) as usize][x as usize] = Some(id);
                        if let Some(b) = self.boxes.get_mut(&id) {
                            b.position.y -= 1;
                        }
                    }
                }
            }
        }

        cleared_lines
    }

    pub fn has_box_at_ceiling(&self) -> bool {
        let ceiling_y = (self.height - 1) as i32;
        (0..self.width as i32).any(|x| {
            if let Some(id) = self.get_box_id_at(x, ceiling_y)
                && let Some(b) = self.boxes.get(&id)
            {
                return !b.falling;
            }
            false
        })
    }

    pub fn boxes(&self) -> impl Iterator<Item = &BoxEntity> {
        self.boxes.values()
    }

    pub fn spawn_box(&mut self, id: u32, x: i32, pattern_id: u32) {
        let y = (self.height - 1) as i32;
        let box_entity = BoxEntity::new(id, Point::new(x, y), pattern_id);
        self.add_box(box_entity);
    }
}
