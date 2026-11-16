use super::candidate::Candidate;
use super::search_board::SearchBoard;
use super::side::Side;
use super::stone::Stone;

/// The legacy minimax bot's one-ply rules, checked in this order before searching: win now,
/// block an immediate loss, make an open `win_count - 1` (a forced win, since the opponent has
/// no immediate win left), stop the opponent's open `win_count - 1`, stop an opponent move that
/// would create two winning cells at once. Each rule takes the first matching candidate in
/// row-major order.
pub(crate) fn tactical_move(
    board: &mut SearchBoard,
    candidates: &[Candidate],
) -> Option<Candidate> {
    let first = |rule: &dyn Fn(&Candidate) -> bool| candidates.iter().copied().find(|c| rule(c));
    first(&|candidate| candidate.probe.bot.completes_line)
        .or_else(|| first(&|candidate| candidate.probe.opponent.completes_line))
        .or_else(|| {
            first_with_stone(board, candidates, Side::Bot, |board, candidate| {
                board.makes_open_threat(candidate.point, Side::Bot)
            })
        })
        .or_else(|| {
            first_with_stone(board, candidates, Side::Opponent, |board, candidate| {
                board.makes_open_threat(candidate.point, Side::Opponent)
            })
        })
        .or_else(|| {
            first_with_stone(board, candidates, Side::Opponent, |board, candidate| {
                board.winning_cells_through(candidate.point, Side::Opponent) >= 2
            })
        })
}

/// First candidate for which `rule` holds with `side`'s stone temporarily placed on it.
fn first_with_stone(
    board: &mut SearchBoard,
    candidates: &[Candidate],
    side: Side,
    rule: impl Fn(&SearchBoard, &Candidate) -> bool,
) -> Option<Candidate> {
    candidates.iter().copied().find(|candidate| {
        board.set_stone(candidate.index, side.stone());
        let holds = rule(board, candidate);
        board.set_stone(candidate.index, Stone::Empty);
        holds
    })
}
