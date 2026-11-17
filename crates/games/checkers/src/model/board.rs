use super::{Piece, PieceId, Rank, Side, Square};

const CELLS: usize = 64;
/// Rows each side fills at the start.
const STARTING_ROWS: u8 = 3;

/// The 8×8 board; only the dark squares (see [`Square`]) ever hold a piece.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Board {
    cells: [Option<Piece>; CELLS],
}

impl Board {
    /// Twelve white men on rows 0–2, twelve black men on rows 5–7.
    pub(crate) fn starting() -> Self {
        let mut board = Self {
            cells: [None; CELLS],
        };
        let mut next_id = 0;
        for square in Square::all() {
            let side = if square.y() < STARTING_ROWS {
                Side::White
            } else if square.y() >= Square::SIZE - STARTING_ROWS {
                Side::Black
            } else {
                continue;
            };
            board.put(
                square,
                Some(Piece {
                    id: PieceId::new(next_id),
                    side,
                    rank: Rank::Man,
                }),
            );
            next_id += 1;
        }
        board
    }

    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self {
            cells: [None; CELLS],
        }
    }

    pub(crate) fn at(&self, square: Square) -> Option<Piece> {
        self.cells.get(square.cell()).copied().flatten()
    }

    pub(crate) fn put(&mut self, square: Square, piece: Option<Piece>) {
        if let Some(cell) = self.cells.get_mut(square.cell()) {
            *cell = piece;
        }
    }

    /// Pieces with their squares, a1 first, row by row.
    pub(crate) fn pieces(&self) -> impl Iterator<Item = (Square, Piece)> + '_ {
        Square::all().filter_map(|square| self.at(square).map(|piece| (square, piece)))
    }

    pub(crate) fn count(&self, side: Side) -> usize {
        self.pieces()
            .filter(|(_, piece)| piece.side == side)
            .count()
    }
}
