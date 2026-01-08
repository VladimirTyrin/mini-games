use std::collections::{HashSet, VecDeque};

use super::types::{DeathReason, Direction, FieldSize, Point};

#[derive(Clone, Debug)]
pub struct Snake {
    pub body: VecDeque<Point>,
    pub body_set: HashSet<Point>,
    pub direction: Direction,
    pub pending_direction: Option<Direction>,
    pub death_reason: Option<DeathReason>,
    pub score: u32,
}

impl Snake {
    pub fn new(start_pos: Point, direction: Direction, field_size: &FieldSize) -> Self {
        let mut body = VecDeque::new();
        let mut body_set = HashSet::new();

        let (dx, dy) = match direction {
            Direction::Up => (0i32, 1i32),
            Direction::Down => (0i32, -1i32),
            Direction::Left => (1i32, 0i32),
            Direction::Right => (-1i32, 0i32),
        };

        let width = field_size.width as i32;
        let height = field_size.height as i32;

        let segment1 = start_pos;
        let segment2 = Point::new(
            ((start_pos.x as i32 + dx + width) % width) as usize,
            ((start_pos.y as i32 + dy + height) % height) as usize,
        );
        let segment3 = Point::new(
            ((segment2.x as i32 + dx + width) % width) as usize,
            ((segment2.y as i32 + dy + height) % height) as usize,
        );

        body.push_back(segment1);
        body.push_back(segment2);
        body.push_back(segment3);

        body_set.insert(segment1);
        body_set.insert(segment2);
        body_set.insert(segment3);

        Self {
            body,
            body_set,
            direction,
            pending_direction: None,
            death_reason: None,
            score: 0,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.death_reason.is_none()
    }

    pub fn head(&self) -> Point {
        *self.body.front().expect("Snake body should never be empty")
    }

    pub fn tail(&self) -> Point {
        *self.body.back().expect("Snake body should never be empty")
    }
}
