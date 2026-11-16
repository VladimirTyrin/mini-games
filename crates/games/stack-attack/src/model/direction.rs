#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Left,
    Right,
}

impl Direction {
    pub const fn dx(self) -> i32 {
        match self {
            Direction::Left => -1,
            Direction::Right => 1,
        }
    }
}
