use crate::model::Cell;

/// What a seat does in the open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// A shot at the opponent's sea, on one's turn.
    Fire(Cell),
    /// Concede: the other seat wins. Allowed at any time, placing included.
    GiveUp,
}
