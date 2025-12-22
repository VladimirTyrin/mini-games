use std::collections::{HashMap, HashSet, VecDeque};
use common::ClientId;
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
    pub alive: bool,
    pub score: u32,
}

impl Snake {
    pub fn new(start_pos: Point, direction: Direction) -> Self {
        let mut body = VecDeque::new();
        let mut body_set = HashSet::new();

        body.push_back(start_pos);
        body_set.insert(start_pos);

        Self {
            body,
            body_set,
            direction,
            pending_direction: None,
            alive: true,
            score: 0,
        }
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
    pub snakes: HashMap<ClientId, Snake>,
    pub food_set: HashSet<Point>,
    pub field_size: FieldSize,
    pub wall_collision_mode: WallCollisionMode,
    pub max_food_count: usize,
    pub food_spawn_probability: f32,
}

impl GameState {
    pub fn new(field_size: FieldSize, wall_collision_mode: WallCollisionMode) -> Self {
        Self {
            snakes: HashMap::new(),
            food_set: HashSet::new(),
            field_size,
            wall_collision_mode,
            max_food_count: 5,
            food_spawn_probability: 0.05,
        }
    }

    fn wrapping_inc(value: usize, max: usize) -> usize {
        if value + 1 >= max {
            0
        } else {
            value + 1
        }
    }

    fn wrapping_dec(value: usize, max: usize) -> usize {
        if value == 0 {
            max - 1
        } else {
            value - 1
        }
    }

    pub fn add_snake(&mut self, client_id: ClientId, start_pos: Point, direction: Direction) {
        let snake = Snake::new(start_pos, direction);
        self.snakes.insert(client_id, snake);
    }

    pub fn remove_snake(&mut self, client_id: &ClientId) {
        self.snakes.remove(client_id);
    }

    pub fn set_snake_direction(&mut self, client_id: &ClientId, direction: Direction) {
        if let Some(snake) = self.snakes.get_mut(client_id) {
            if snake.alive && !direction.is_opposite(&snake.direction) {
                snake.pending_direction = Some(direction);
            }
        }
    }

    pub fn update(&mut self) {
        self.try_spawn_food();

        for snake in self.snakes.values_mut() {
            if !snake.alive {
                continue;
            }

            if let Some(new_direction) = snake.pending_direction {
                snake.direction = new_direction;
                snake.pending_direction = None;
            }
        }

        let client_ids: Vec<ClientId> = self.snakes.keys().cloned().collect();

        for client_id in client_ids {
            let snake = self.snakes.get_mut(&client_id).unwrap();
            if !snake.alive {
                continue;
            }

            match self.try_move_snake_for_client(&client_id) {
                Ok(_) => {},
                Err(_) => {
                    let snake = self.snakes.get_mut(&client_id).unwrap();
                    snake.alive = false;
                }
            }
        }
    }

    fn try_move_snake_for_client(&mut self, client_id: &ClientId) -> Result<(), String> {
        let next_head = {
            let snake = self.snakes.get(client_id).unwrap();
            self.calculate_next_head_position_for_client(client_id, snake)?
        };

        let snake = self.snakes.get_mut(client_id).unwrap();
        snake.body.push_front(next_head);
        snake.body_set.insert(next_head);

        if self.food_set.contains(&next_head) {
            self.food_set.remove(&next_head);
            snake.score += 1;
        } else {
            let tail = snake.body.pop_back().unwrap();
            snake.body_set.remove(&tail);
        }

        Ok(())
    }

    fn calculate_next_head_position_for_client(&self, _client_id: &ClientId, snake: &Snake) -> Result<Point, String> {
        let head = snake.head();
        let direction = &snake.direction;

        let next_head = match self.wall_collision_mode {
            WallCollisionMode::Death => {
                match direction {
                    Direction::Up => {
                        if head.y == 0 {
                            return Err("Wall collision".to_string());
                        }
                        Point::new(head.x, head.y - 1)
                    }
                    Direction::Down => {
                        if head.y >= self.field_size.height - 1 {
                            return Err("Wall collision".to_string());
                        }
                        Point::new(head.x, head.y + 1)
                    }
                    Direction::Left => {
                        if head.x == 0 {
                            return Err("Wall collision".to_string());
                        }
                        Point::new(head.x - 1, head.y)
                    }
                    Direction::Right => {
                        if head.x >= self.field_size.width - 1 {
                            return Err("Wall collision".to_string());
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
            return Err("Self collision".to_string());
        }

        for (_other_id, other_snake) in &self.snakes {
            if !other_snake.alive {
                continue;
            }

            if other_snake.body_set.contains(&next_head) {
                return Err("Collision with another snake".to_string());
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
