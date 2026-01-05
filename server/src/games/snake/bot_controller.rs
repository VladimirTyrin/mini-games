use common::{PlayerId, SnakeBotType};
use super::game_state::{GameState, Direction, Point, WallCollisionMode, DeadSnakeBehavior};
use rand::Rng;

pub struct BotController;

impl BotController {
    pub fn calculate_move(
        bot_type: SnakeBotType,
        player_id: &PlayerId,
        state: &GameState,
    ) -> Option<Direction> {
        match bot_type {
            SnakeBotType::Efficient => Self::efficient_pathfinding(player_id, state),
            SnakeBotType::Random => Self::random_valid_move(player_id, state),
            SnakeBotType::Unspecified => None,
        }
    }

    fn efficient_pathfinding(player_id: &PlayerId, state: &GameState) -> Option<Direction> {
        let snake = state.snakes.get(player_id)?;
        if !snake.is_alive() {
            return None;
        }

        let head = snake.head();
        let current_direction = snake.direction;

        let nearest_food = Self::find_nearest_food(head, state)?;

        let valid_directions = Self::get_valid_directions(current_direction);

        let mut best_dir = None;
        let mut best_distance = f32::MAX;

        for dir in valid_directions {
            if let Some(next_pos) = Self::calculate_next_position(head, dir, state)
                && Self::is_safe_position(next_pos, player_id, state) {
                    let distance = Self::manhattan_distance(next_pos, nearest_food, state);
                    if distance < best_distance {
                        best_distance = distance;
                        best_dir = Some(dir);
                    }
                }
        }

        best_dir.or_else(|| Self::random_valid_move(player_id, state))
    }

    fn random_valid_move(player_id: &PlayerId, state: &GameState) -> Option<Direction> {
        let snake = state.snakes.get(player_id)?;
        if !snake.is_alive() {
            return None;
        }

        let current_direction = snake.direction;
        let valid_directions = Self::get_valid_directions(current_direction);

        let head = snake.head();
        let safe_directions: Vec<Direction> = valid_directions
            .into_iter()
            .filter(|&dir| {
                if let Some(next_pos) = Self::calculate_next_position(head, dir, state) {
                    Self::is_safe_position(next_pos, player_id, state)
                } else {
                    false
                }
            })
            .collect();

        if safe_directions.is_empty() {
            Some(current_direction)
        } else {
            let mut rng = rand::rng();
            let idx = rng.random_range(0..safe_directions.len());
            Some(safe_directions[idx])
        }
    }

    fn get_valid_directions(current: Direction) -> Vec<Direction> {
        vec![Direction::Up, Direction::Down, Direction::Left, Direction::Right]
            .into_iter()
            .filter(|d| !d.is_opposite(&current))
            .collect()
    }

    fn find_nearest_food(from: Point, state: &GameState) -> Option<Point> {
        state.food_set.iter()
            .min_by_key(|food| Self::manhattan_distance(from, **food, state) as i32)
            .copied()
    }

    fn manhattan_distance(a: Point, b: Point, state: &GameState) -> f32 {
        let dx = (a.x as i32 - b.x as i32).abs();
        let dy = (a.y as i32 - b.y as i32).abs();

        match state.wall_collision_mode {
            WallCollisionMode::Death => (dx + dy) as f32,
            WallCollisionMode::WrapAround => {
                let width = state.field_size.width as i32;
                let height = state.field_size.height as i32;

                let wrapped_dx = width - dx;
                let wrapped_dy = height - dy;

                let min_dx = dx.min(wrapped_dx);
                let min_dy = dy.min(wrapped_dy);

                (min_dx + min_dy) as f32
            }
        }
    }

    fn calculate_next_position(
        from: Point,
        direction: Direction,
        state: &GameState,
    ) -> Option<Point> {
        match state.wall_collision_mode {
            WallCollisionMode::Death => {
                match direction {
                    Direction::Up if from.y > 0 => Some(Point::new(from.x, from.y - 1)),
                    Direction::Down if from.y < state.field_size.height - 1 => Some(Point::new(from.x, from.y + 1)),
                    Direction::Left if from.x > 0 => Some(Point::new(from.x - 1, from.y)),
                    Direction::Right if from.x < state.field_size.width - 1 => Some(Point::new(from.x + 1, from.y)),
                    _ => None,
                }
            }
            WallCollisionMode::WrapAround => {
                match direction {
                    Direction::Up => Some(Point::new(from.x, GameState::wrapping_dec(from.y, state.field_size.height))),
                    Direction::Down => Some(Point::new(from.x, GameState::wrapping_inc(from.y, state.field_size.height))),
                    Direction::Left => Some(Point::new(GameState::wrapping_dec(from.x, state.field_size.width), from.y)),
                    Direction::Right => Some(Point::new(GameState::wrapping_inc(from.x, state.field_size.width), from.y)),
                }
            }
        }
    }

    fn is_safe_position(pos: Point, player_id: &PlayerId, state: &GameState) -> bool {
        for (id, snake) in &state.snakes {
            let should_check = match state.dead_snake_behavior {
                DeadSnakeBehavior::Disappear => snake.is_alive(),
                DeadSnakeBehavior::StayOnField => true,
            };

            if !should_check {
                continue;
            }

            if id == player_id {
                if snake.is_alive() && snake.body_set.contains(&pos) && pos != snake.tail() {
                    return false;
                }
            } else if snake.body_set.contains(&pos) {
                return false;
            }
        }
        true
    }
}
