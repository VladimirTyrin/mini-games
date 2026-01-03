use std::collections::{HashMap, HashSet, VecDeque};
use common::{log, PlayerId};
use rand::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

impl Point {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WallCollisionMode {
    Death,
    WrapAround,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathReason {
    WallCollision,
    SelfCollision,
    OtherSnakeCollision,
    PlayerDisconnected,
}

impl Direction {
    pub fn is_opposite(&self, other: &Direction) -> bool {
        matches!(
            (self, other),
            (Direction::Left, Direction::Right)
                | (Direction::Right, Direction::Left)
                | (Direction::Up, Direction::Down)
                | (Direction::Down, Direction::Up)
        )
    }
}

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
        *self.body.front().unwrap()
    }

    pub fn tail(&self) -> Point {
        *self.body.back().unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct FieldSize {
    pub width: usize,
    pub height: usize,
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub snakes: HashMap<PlayerId, Snake>,
    pub food_set: HashSet<Point>,
    pub field_size: FieldSize,
    pub wall_collision_mode: WallCollisionMode,
    pub max_food_count: usize,
    pub food_spawn_probability: f32,
    pub game_end_reason: Option<DeathReason>,
}

impl GameState {
    pub fn new(field_size: FieldSize, wall_collision_mode: WallCollisionMode, max_food_count: usize, food_spawn_probability: f32) -> Self {
        Self {
            snakes: HashMap::new(),
            food_set: HashSet::new(),
            field_size,
            wall_collision_mode,
            max_food_count,
            food_spawn_probability,
            game_end_reason: None,
        }
    }

    pub fn wrapping_inc(value: usize, max: usize) -> usize {
        if value + 1 >= max {
            0
        } else {
            value + 1
        }
    }

    pub fn wrapping_dec(value: usize, max: usize) -> usize {
        if value == 0 {
            max - 1
        } else {
            value - 1
        }
    }

    pub fn add_snake(&mut self, player_id: PlayerId, start_pos: Point, direction: Direction) {
        let snake = Snake::new(start_pos, direction, &self.field_size);
        self.snakes.insert(player_id, snake);
    }

    pub fn kill_snake(&mut self, player_id: &PlayerId, reason: DeathReason) {
        if let Some(snake) = self.snakes.get_mut(player_id) {
            if snake.is_alive() {
                snake.death_reason = Some(reason);
                self.game_end_reason = Some(reason);
            }
        }
    }

    pub fn set_snake_direction(&mut self, player_id: &PlayerId, direction: Direction) {
        if let Some(snake) = self.snakes.get_mut(player_id) {
            if snake.is_alive() && !direction.is_opposite(&snake.direction) {
                snake.pending_direction = Some(direction);
            }
        }
    }

    pub fn update(&mut self) {
        self.try_spawn_food();

        for snake in self.snakes.values_mut() {
            if !snake.is_alive() {
                continue;
            }

            if let Some(new_direction) = snake.pending_direction {
                snake.direction = new_direction;
                snake.pending_direction = None;
            }
        }

        let player_ids: Vec<PlayerId> = self.snakes.keys().cloned().collect();

        for player_id in player_ids {
            let snake = self.snakes.get_mut(&player_id).unwrap();
            if !snake.is_alive() {
                continue;
            }

            match self.try_move_snake_for_player(&player_id) {
                Ok(_) => {},
                Err(reason) => {
                    let snake = self.snakes.get_mut(&player_id).unwrap();
                    snake.death_reason = Some(reason);
                    self.game_end_reason = Some(reason);
                }
            }
        }
    }

    fn try_move_snake_for_player(&mut self, player_id: &PlayerId) -> Result<(), DeathReason> {
        let next_head = {
            let snake = self.snakes.get(player_id).unwrap();
            self.calculate_next_head_position_for_player(player_id, snake)?
        };

        let snake = self.snakes.get_mut(player_id).unwrap();
        snake.body.push_front(next_head);
        snake.body_set.insert(next_head);

        if self.food_set.contains(&next_head) {
            self.food_set.remove(&next_head);
            snake.score += 1;
            log!("[{}] ate food at ({}, {}). Score: {}", player_id, next_head.x, next_head.y, snake.score);
        } else {
            let tail = snake.body.pop_back().unwrap();
            snake.body_set.remove(&tail);
        }

        Ok(())
    }

    fn calculate_next_head_position_for_player(&self, player_id: &PlayerId, snake: &Snake) -> Result<Point, DeathReason> {
        let head = snake.head();
        let direction = &snake.direction;

        let next_head = match self.wall_collision_mode {
            WallCollisionMode::Death => {
                match direction {
                    Direction::Up => {
                        if head.y == 0 {
                            return Err(DeathReason::WallCollision);
                        }
                        Point::new(head.x, head.y - 1)
                    }
                    Direction::Down => {
                        if head.y >= self.field_size.height - 1 {
                            return Err(DeathReason::WallCollision);
                        }
                        Point::new(head.x, head.y + 1)
                    }
                    Direction::Left => {
                        if head.x == 0 {
                            return Err(DeathReason::WallCollision);
                        }
                        Point::new(head.x - 1, head.y)
                    }
                    Direction::Right => {
                        if head.x >= self.field_size.width - 1 {
                            return Err(DeathReason::WallCollision);
                        }
                        Point::new(head.x + 1, head.y)
                    }
                }
            }
            WallCollisionMode::WrapAround => {
                match direction {
                    Direction::Up => Point::new(head.x, Self::wrapping_dec(head.y, self.field_size.height)),
                    Direction::Down => Point::new(head.x, Self::wrapping_inc(head.y, self.field_size.height)),
                    Direction::Left => Point::new(Self::wrapping_dec(head.x, self.field_size.width), head.y),
                    Direction::Right => Point::new(Self::wrapping_inc(head.x, self.field_size.width), head.y),
                }
            }
        };

        if snake.body_set.contains(&next_head) && next_head != snake.tail() {
            return Err(DeathReason::SelfCollision);
        }

        for (other_id, other_snake) in &self.snakes {
            if other_id == player_id {
                continue;
            }

            if !other_snake.is_alive() {
                continue;
            }

            if other_snake.body_set.contains(&next_head) {
                log!("{} collided with {} at ({}, {})", player_id, other_id, next_head.x, next_head.y);
                return Err(DeathReason::OtherSnakeCollision);
            }
        }

        Ok(next_head)
    }

    fn try_spawn_food(&mut self) {
        if !self.should_spawn_food() {
            return;
        }

        let mut rng = rand::rng();

        for _ in 0..100 {
            let x = rng.random_range(0..self.field_size.width);
            let y = rng.random_range(0..self.field_size.height);
            let pos = Point::new(x, y);

            if self.food_set.contains(&pos) {
                continue;
            }

            let mut occupied = false;
            for snake in self.snakes.values() {
                if snake.body_set.contains(&pos) {
                    occupied = true;
                    break;
                }
            }

            if !occupied {
                self.food_set.insert(pos);
                log!("Food spawned at ({}, {})", pos.x, pos.y);
                return;
            }
        }
    }

    fn should_spawn_food(&self) -> bool {
        if self.food_set.len() >= self.max_food_count {
            return false;
        }
        let mut rng = rand::rng();
        rng.random::<f32>() < self.food_spawn_probability
    }
}
