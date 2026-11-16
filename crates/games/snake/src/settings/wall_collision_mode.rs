#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WallCollisionMode {
    /// Leaving the field kills the snake.
    Death,
    /// The head re-enters on the opposite side.
    WrapAround,
}

impl WallCollisionMode {
    pub(crate) const fn hash_code(self) -> u8 {
        match self {
            WallCollisionMode::Death => 0,
            WallCollisionMode::WrapAround => 1,
        }
    }
}
