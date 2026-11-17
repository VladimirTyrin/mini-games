use super::{PieceId, Rank, Side};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Piece {
    pub id: PieceId,
    pub side: Side,
    pub rank: Rank,
}

impl Piece {
    pub(crate) fn code(self) -> u8 {
        self.side.code() * 2 + u8::from(self.rank == Rank::King)
    }
}
