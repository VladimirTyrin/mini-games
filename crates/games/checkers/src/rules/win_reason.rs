#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WinReason {
    /// The loser has no pieces left.
    NoPieces,
    /// The loser cannot move (all blocked).
    NoMoves,
    /// The loser left the game.
    OpponentLeft,
}

impl WinReason {
    pub(crate) fn code(self) -> u8 {
        match self {
            WinReason::NoPieces => 1,
            WinReason::NoMoves => 2,
            WinReason::OpponentLeft => 3,
        }
    }
}
