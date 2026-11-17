use crate::model::{Board, Side, Square};

/// Identifies a position for the repetition rule: what stands where and whose move it is.
/// Piece ids do not count. FNV-1a: fixed, so every build agrees.
pub(crate) fn position_key(board: &Board, to_move: Side) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut key = OFFSET;
    let mut feed = |byte: u8| {
        key ^= u64::from(byte);
        key = key.wrapping_mul(PRIME);
    };
    for square in Square::all() {
        feed(board.at(square).map_or(0, |piece| piece.code()));
    }
    feed(to_move.code());
    key
}
