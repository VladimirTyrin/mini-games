#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeathReason {
    WallCollision,
    SelfCollision,
    /// Another snake's body (dead bodies included with `StayOnField`), a head-on collision or a
    /// swap of cells.
    SnakeCollision,
    /// The seat was dropped (left, kicked, disconnect budget exhausted).
    PlayerDisconnected,
}

impl DeathReason {
    pub(crate) const fn hash_code(self) -> u8 {
        match self {
            DeathReason::WallCollision => 1,
            DeathReason::SelfCollision => 2,
            DeathReason::SnakeCollision => 3,
            DeathReason::PlayerDisconnected => 4,
        }
    }
}
