//! Russian draughts moves. Capturing is compulsory, but any capture sequence will do (not
//! necessarily the longest). Captured pieces stay on the board until the move is over: they
//! block, and none can be jumped twice. A man that reaches the far row while capturing goes on
//! as a king. A king may land on any empty square beyond the piece it jumps, but must land
//! where it can capture again when there is such a square.

use crate::model::{Board, Direction, Path, Rank, Side, Square};
use crate::rules::LegalMove;

/// Every legal move of `side`, in a fixed order (squares a1 first, then directions).
pub(crate) fn legal_moves(board: &Board, side: Side) -> Vec<LegalMove> {
    let mut captures = Vec::new();
    for (square, piece) in board.pieces() {
        if piece.side == side {
            CaptureSearch::new(board, side, square).collect(piece.rank, &mut captures);
        }
    }
    if !captures.is_empty() {
        return captures;
    }
    let mut moves = Vec::new();
    for (square, piece) in board.pieces() {
        if piece.side == side {
            simple_moves(board, side, square, piece.rank, &mut moves);
        }
    }
    moves
}

fn simple_moves(board: &Board, side: Side, from: Square, rank: Rank, out: &mut Vec<LegalMove>) {
    for direction in Direction::ALL {
        if rank == Rank::Man && direction.dy() != side.forward() {
            continue;
        }
        let mut to = from.step(direction);
        while let Some(square) = to {
            if board.at(square).is_some() {
                break;
            }
            if let Some(path) = Path::new(vec![from, square]) {
                out.push(LegalMove {
                    path,
                    captured: Vec::new(),
                    crowns: rank == Rank::Man && square.y() == side.crowning_row(),
                });
            }
            if rank == Rank::Man {
                break;
            }
            to = square.step(direction);
        }
    }
}

/// One jump of a capture: the piece taken and where the capturing piece may land.
struct Jump {
    victim: Square,
    landings: Vec<Square>,
}

struct CaptureSearch<'board> {
    board: &'board Board,
    side: Side,
    /// The moving piece has left it: it counts as empty during the move.
    origin: Square,
}

impl<'board> CaptureSearch<'board> {
    fn new(board: &'board Board, side: Side, origin: Square) -> Self {
        Self {
            board,
            side,
            origin,
        }
    }

    fn collect(&self, rank: Rank, out: &mut Vec<LegalMove>) {
        let mut path = vec![self.origin];
        let mut captured = Vec::new();
        self.extend(self.origin, rank, rank, &mut path, &mut captured, out);
    }

    fn is_empty(&self, square: Square) -> bool {
        square == self.origin || self.board.at(square).is_none()
    }

    fn is_victim(&self, square: Square, captured: &[Square]) -> bool {
        square != self.origin
            && !captured.contains(&square)
            && self
                .board
                .at(square)
                .is_some_and(|piece| piece.side != self.side)
    }

    fn jumps(&self, from: Square, rank: Rank, captured: &[Square]) -> Vec<Jump> {
        let mut jumps = Vec::new();
        for direction in Direction::ALL {
            // A man jumps the adjacent square; a king flies over empty squares to the first piece.
            let mut victim = from.step(direction);
            if rank == Rank::King {
                while let Some(square) = victim.filter(|square| self.is_empty(*square)) {
                    victim = square.step(direction);
                }
            }
            let Some(victim) = victim.filter(|square| self.is_victim(*square, captured)) else {
                continue;
            };
            let mut landings = Vec::new();
            let mut landing = victim.step(direction);
            while let Some(square) = landing.filter(|square| self.is_empty(*square)) {
                landings.push(square);
                if rank == Rank::Man {
                    break;
                }
                landing = square.step(direction);
            }
            if !landings.is_empty() {
                jumps.push(Jump { victim, landings });
            }
        }
        jumps
    }

    fn rank_after_landing(&self, rank: Rank, landing: Square) -> Rank {
        if landing.y() == self.side.crowning_row() {
            Rank::King
        } else {
            rank
        }
    }

    fn extend(
        &self,
        from: Square,
        start_rank: Rank,
        rank: Rank,
        path: &mut Vec<Square>,
        captured: &mut Vec<Square>,
        out: &mut Vec<LegalMove>,
    ) {
        let jumps = self.jumps(from, rank, captured);
        if jumps.is_empty() {
            if !captured.is_empty()
                && let Some(complete) = Path::new(path.clone())
            {
                out.push(LegalMove {
                    path: complete,
                    captured: captured.clone(),
                    crowns: start_rank == Rank::Man && rank == Rank::King,
                });
            }
            return;
        }
        for jump in jumps {
            captured.push(jump.victim);
            let continuing: Vec<Square> = jump
                .landings
                .iter()
                .copied()
                .filter(|landing| {
                    let next = self.rank_after_landing(rank, *landing);
                    !self.jumps(*landing, next, captured).is_empty()
                })
                .collect();
            // Where the capture can go on, it must.
            let landings = if continuing.is_empty() {
                jump.landings
            } else {
                continuing
            };
            for landing in landings {
                path.push(landing);
                let next = self.rank_after_landing(rank, landing);
                self.extend(landing, start_rank, next, path, captured, out);
                path.pop();
            }
            captured.pop();
        }
    }
}
