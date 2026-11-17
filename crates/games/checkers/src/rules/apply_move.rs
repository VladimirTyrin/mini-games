use crate::model::{Board, Rank};
use crate::rules::LegalMove;

/// Moves the piece along `chosen`, removes what it captured and crowns it if it got there.
/// `chosen` must be legal on `board`.
pub(crate) fn apply_move(board: &mut Board, chosen: &LegalMove) {
    let (Some(from), Some(to)) = (chosen.path.from(), chosen.path.to()) else {
        return;
    };
    let Some(mut piece) = board.at(from) else {
        return;
    };
    board.put(from, None);
    for victim in &chosen.captured {
        board.put(*victim, None);
    }
    if chosen.crowns {
        piece.rank = Rank::King;
    }
    board.put(to, Some(piece));
}
