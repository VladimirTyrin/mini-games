use crate::direction::Direction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Rejected with `CommandError::Illegal` if it would not change the board.
    Move(Direction),
}
