use crate::model::{Piece, Square};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedPiece {
    pub square: Square,
    pub piece: Piece,
}
