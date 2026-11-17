use mg_core::Slot;

use super::PlacedPiece;
use crate::model::Side;
use crate::rules::{GameEnd, LegalMove};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub pieces: Vec<PlacedPiece>,
    /// `None` until drawn by lot.
    pub white_player: Option<Slot>,
    pub black_player: Option<Slot>,
    /// `None` before white is known and after the game ended.
    pub current_player: Option<Slot>,
    pub side_to_move: Side,
    pub you: Option<Slot>,
    /// What the side to move may do: the board offers these and nothing else.
    pub legal_moves: Vec<LegalMove>,
    /// For the animation: the move just made.
    pub last_move: Option<LegalMove>,
    pub end: Option<GameEnd>,
    /// Moves in a row with only kings moving (see `KING_MOVES_FOR_DRAW`).
    pub king_moves: u8,
}
