use crate::model::{Board, Rank, Side, Square};

const MAN: i32 = 100;
/// A flying king is worth about three men in Russian draughts.
const KING: i32 = 300;
/// Per row a man has advanced: closer to crowning.
const ADVANCE: i32 = 4;
/// A man still on its home row keeps the opponent from crowning there.
const HOME_ROW_GUARD: i32 = 6;
const CENTRE: i32 = 3;
/// The long diagonal a1–h8 ("большак"): a king there controls the most squares.
const LONG_DIAGONAL: i32 = 8;

/// The position from `side`'s point of view: positive is good for it.
// Not inlined so that `cargo asm -p mg-checkers --lib evaluate` can show it on its own; the
// call costs a few nanoseconds against the tens the evaluation itself takes.
#[inline(never)]
pub(crate) fn evaluate(board: &Board, side: Side) -> i32 {
    board
        .pieces()
        .map(|(square, piece)| {
            let worth = match piece.rank {
                Rank::Man => MAN + man_bonus(square, piece.side),
                Rank::King => {
                    KING + if square.x() == square.y() {
                        LONG_DIAGONAL
                    } else {
                        0
                    }
                }
            };
            if piece.side == side { worth } else { -worth }
        })
        .sum()
}

fn man_bonus(square: Square, side: Side) -> i32 {
    let advanced = match side {
        Side::White => square.y(),
        Side::Black => Square::SIZE - 1 - square.y(),
    };
    let home_row = if advanced == 0 { HOME_ROW_GUARD } else { 0 };
    let centre = if (2..=5).contains(&square.x()) && (2..=5).contains(&square.y()) {
        CENTRE
    } else {
        0
    };
    i32::from(advanced) * ADVANCE + home_row + centre
}
