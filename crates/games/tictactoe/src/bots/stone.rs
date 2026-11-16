#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stone {
    Empty,
    Bot,
    Opponent,
    /// Returned for coordinates outside the board so line scans need no bounds checks.
    OffBoard,
}
