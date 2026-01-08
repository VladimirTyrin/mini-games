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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WallCollisionMode {
    Death,
    WrapAround,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeadSnakeBehavior {
    Disappear,
    StayOnField,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathReason {
    WallCollision,
    SelfCollision,
    OtherSnakeCollision,
    PlayerDisconnected,
}

#[derive(Clone, Debug)]
pub struct FieldSize {
    pub width: usize,
    pub height: usize,
}
