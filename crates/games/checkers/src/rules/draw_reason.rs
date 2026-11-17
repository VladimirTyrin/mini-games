#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawReason {
    /// The same position, with the same side to move, came up a third time.
    Repetition,
    /// Fifteen moves each with kings only: no capture, no man moved.
    KingMoves,
}

impl DrawReason {
    pub(crate) fn code(self) -> u8 {
        match self {
            DrawReason::Repetition => 4,
            DrawReason::KingMoves => 5,
        }
    }
}
