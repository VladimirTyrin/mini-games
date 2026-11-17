use super::Square;

/// The squares a piece passes through in one move: where it starts, then each square it lands
/// on (one for a simple move, one per captured piece for a capture). Valid by construction:
/// 2..=13 squares (at most 12 pieces can be captured in one move).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Path(Vec<Square>);

impl Path {
    pub const MAX_SQUARES: usize = 13;

    pub fn new(squares: Vec<Square>) -> Option<Self> {
        (2..=Self::MAX_SQUARES)
            .contains(&squares.len())
            .then_some(Self(squares))
    }

    pub fn squares(&self) -> &[Square] {
        &self.0
    }

    pub fn from(&self) -> Option<Square> {
        self.0.first().copied()
    }

    pub fn to(&self) -> Option<Square> {
        self.0.last().copied()
    }
}
