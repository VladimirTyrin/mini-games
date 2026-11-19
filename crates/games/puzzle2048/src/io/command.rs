use crate::model::Direction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Rejected with `Refusal::MoveChangesNothing` if it would not change the board.
    Move(Direction),
}
