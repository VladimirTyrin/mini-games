use std::collections::{HashMap, HashSet};

use crate::{log, PlayerId};
use crate::games::SessionRng;
use super::snake::Snake;
use super::types::{DeadSnakeBehavior, DeathReason, Direction, FieldSize, Point, WallCollisionMode};

#[derive(Clone, Debug)]
pub struct SnakeGameState {
    pub snakes: HashMap<PlayerId, Snake>,
    pub food_set: HashSet<Point>,
    pub field_size: FieldSize,
    pub wall_collision_mode: WallCollisionMode,
    pub dead_snake_behavior: DeadSnakeBehavior,
    pub max_food_count: usize,
    pub food_spawn_probability: f32,
    pub game_end_reason: Option<DeathReason>,
    player_order: Vec<PlayerId>,
}

impl SnakeGameState {
    pub fn new(
        field_size: FieldSize,
        wall_collision_mode: WallCollisionMode,
        dead_snake_behavior: DeadSnakeBehavior,
        max_food_count: usize,
        food_spawn_probability: f32,
    ) -> Self {
        Self {
            snakes: HashMap::new(),
            food_set: HashSet::new(),
            field_size,
            wall_collision_mode,
            dead_snake_behavior,
            max_food_count,
            food_spawn_probability,
            game_end_reason: None,
            player_order: Vec::new(),
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
        self.snakes.insert(player_id.clone(), snake);
        self.player_order.push(player_id);
        self.player_order.sort();
    }

    pub fn kill_snake(&mut self, player_id: &PlayerId, reason: DeathReason) {
        if let Some(snake) = self.snakes.get_mut(player_id)
            && snake.is_alive()
        {
            snake.death_reason = Some(reason);
            self.game_end_reason = Some(reason);
        }
    }

    pub fn set_snake_direction(&mut self, player_id: &PlayerId, direction: Direction) {
        if let Some(snake) = self.snakes.get_mut(player_id)
            && snake.is_alive()
            && !direction.is_opposite(&snake.direction)
        {
            snake.pending_direction = Some(direction);
        }
    }

    pub fn update(&mut self, rng: &mut SessionRng) {
        self.try_spawn_food(rng);

        for snake in self.snakes.values_mut() {
            if !snake.is_alive() {
                continue;
            }

            if let Some(new_direction) = snake.pending_direction {
                snake.direction = new_direction;
                snake.pending_direction = None;
            }
        }

        for player_id in self.player_order.clone() {
            let snake = self
                .snakes
                .get_mut(&player_id)
                .expect("Player ID should exist in snakes map");
            if !snake.is_alive() {
                continue;
            }

            match self.try_move_snake_for_player(&player_id) {
                Ok(_) => {}
                Err(reason) => {
                    let snake = self
                        .snakes
                        .get_mut(&player_id)
                        .expect("Player ID should exist in snakes map");
                    snake.death_reason = Some(reason);
                    self.game_end_reason = Some(reason);
                }
            }
        }
    }

    fn try_move_snake_for_player(&mut self, player_id: &PlayerId) -> Result<(), DeathReason> {
        let next_head = {
            let snake = self
                .snakes
                .get(player_id)
                .expect("Player ID should exist in snakes map");
            self.calculate_next_head_position_for_player(player_id, snake)?
        };

        let snake = self
            .snakes
            .get_mut(player_id)
            .expect("Player ID should exist in snakes map");
        snake.body.push_front(next_head);
        snake.body_set.insert(next_head);

        if self.food_set.contains(&next_head) {
            self.food_set.remove(&next_head);
            snake.score += 1;
            log!(
                "[{}] ate food at ({}, {}). Score: {}",
                player_id,
                next_head.x,
                next_head.y,
                snake.score
            );
        } else {
            let tail = snake
                .body
                .pop_back()
                .expect("Snake body should never be empty");
            snake.body_set.remove(&tail);
        }

        Ok(())
    }

    fn calculate_next_head_position_for_player(
        &self,
        player_id: &PlayerId,
        snake: &Snake,
    ) -> Result<Point, DeathReason> {
        let head = snake.head();
        let direction = &snake.direction;

        let next_head = match self.wall_collision_mode {
            WallCollisionMode::Death => match direction {
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
            },
            WallCollisionMode::WrapAround => match direction {
                Direction::Up => {
                    Point::new(head.x, Self::wrapping_dec(head.y, self.field_size.height))
                }
                Direction::Down => {
                    Point::new(head.x, Self::wrapping_inc(head.y, self.field_size.height))
                }
                Direction::Left => {
                    Point::new(Self::wrapping_dec(head.x, self.field_size.width), head.y)
                }
                Direction::Right => {
                    Point::new(Self::wrapping_inc(head.x, self.field_size.width), head.y)
                }
            },
        };

        if snake.body_set.contains(&next_head) && next_head != snake.tail() {
            return Err(DeathReason::SelfCollision);
        }

        for (other_id, other_snake) in &self.snakes {
            if other_id == player_id {
                continue;
            }

            let should_check_collision = match self.dead_snake_behavior {
                DeadSnakeBehavior::Disappear => other_snake.is_alive(),
                DeadSnakeBehavior::StayOnField => true,
            };

            if should_check_collision && other_snake.body_set.contains(&next_head) {
                log!(
                    "{} collided with {} at ({}, {})",
                    player_id,
                    other_id,
                    next_head.x,
                    next_head.y
                );
                return Err(DeathReason::OtherSnakeCollision);
            }
        }

        Ok(next_head)
    }

    fn try_spawn_food(&mut self, rng: &mut SessionRng) {
        if !self.should_spawn_food(rng) {
            return;
        }

        for _ in 0..100 {
            let x = rng.random_range(0..self.field_size.width);
            let y = rng.random_range(0..self.field_size.height);
            let pos = Point::new(x, y);

            if self.food_set.contains(&pos) {
                continue;
            }

            let mut occupied = false;
            for snake in self.snakes.values() {
                let should_check_occupied = match self.dead_snake_behavior {
                    DeadSnakeBehavior::Disappear => snake.is_alive(),
                    DeadSnakeBehavior::StayOnField => true,
                };

                if should_check_occupied && snake.body_set.contains(&pos) {
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

    fn should_spawn_food(&self, rng: &mut SessionRng) -> bool {
        if self.food_set.len() >= self.max_food_count {
            return false;
        }
        rng.random::<f32>() < self.food_spawn_probability
    }
}
