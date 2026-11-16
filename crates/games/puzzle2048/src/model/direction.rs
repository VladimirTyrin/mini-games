#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Towards row 0.
    Up,
    Down,
    /// Towards column 0.
    Left,
    Right,
}

impl Direction {
    pub const ALL: [Direction; 4] = [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ];

    /// Feeds the state hash: never renumber.
    pub(crate) fn code(self) -> u8 {
        match self {
            Direction::Up => 0,
            Direction::Down => 1,
            Direction::Left => 2,
            Direction::Right => 3,
        }
    }
}
