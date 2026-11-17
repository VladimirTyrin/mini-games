/// Stable over the game, so the board can animate a piece from square to square.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PieceId(u8);

impl PieceId {
    pub(crate) fn new(id: u8) -> Self {
        Self(id)
    }

    pub fn get(self) -> u8 {
        self.0
    }
}
