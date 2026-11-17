use crate::model::{Path, Square};

/// A move the side to move may make, with what it does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegalMove {
    pub path: Path,
    /// Squares of the pieces it captures, in the order they are jumped.
    pub captured: Vec<Square>,
    /// The moving man ends as a king (it reached the far row, maybe in the middle of a capture).
    pub crowns: bool,
}
