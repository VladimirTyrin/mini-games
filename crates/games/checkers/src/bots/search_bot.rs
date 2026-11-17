use mg_core::{Bot, Budget, CoreRng, Slot};

use super::evaluation::evaluate;
use super::{Bound, SearchMemory, TableEntry, TranspositionTable};
use crate::game::Checkers;
use crate::io::Command;
use crate::model::{Board, Side};
use crate::rules::{LegalMove, apply_move, legal_moves, position_key};

/// Beats any evaluation: the side to move cannot move (no pieces or all blocked).
const LOSS: i32 = -1_000_000;
/// Extra plies searched past the horizon while captures are pending: stopping in the middle of
/// an exchange misjudges it badly.
const CAPTURE_EXTENSION: u8 = 6;

/// Negamax with alpha-beta, deepened one ply at a time until `max_depth` or the budget runs
/// out; the last finished depth decides. Equally good moves are picked at random.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SearchBot {
    max_depth: u8,
    memory: SearchMemory,
}

impl SearchBot {
    pub(crate) fn new(max_depth: u8, memory: SearchMemory) -> Self {
        Self { max_depth, memory }
    }
}

/// Out of budget: the search in progress is abandoned.
struct OutOfBudget;

impl Bot<Checkers> for SearchBot {
    fn decide(
        &mut self,
        game: &Checkers,
        me: Slot,
        rng: &mut CoreRng,
        budget: &mut Budget,
    ) -> Option<Command> {
        if game.current_player() != Some(me) {
            return None;
        }
        let side = game.to_move();
        let moves = game.legal_moves();
        if moves.len() == 1 {
            return moves.first().map(|only| Command::Move(only.path.clone()));
        }
        let mut search = Search {
            budget,
            table: match self.memory {
                SearchMemory::None => None,
                SearchMemory::TranspositionTable => Some(TranspositionTable::new()),
            },
        };
        let mut best: Vec<&LegalMove> = moves.iter().collect();
        for depth in 1..=self.max_depth {
            match search.root(game.board(), side, &moves, depth) {
                Ok(found) => best = found,
                Err(OutOfBudget) => break,
            }
        }
        rng.pick(&best)
            .map(|chosen| Command::Move(chosen.path.clone()))
    }
}

struct Search<'budget> {
    budget: &'budget mut Budget,
    table: Option<TranspositionTable>,
}

impl Search<'_> {
    /// The best moves at `depth`, all of equal score.
    fn root<'moves>(
        &mut self,
        board: &Board,
        side: Side,
        moves: &'moves [LegalMove],
        depth: u8,
    ) -> Result<Vec<&'moves LegalMove>, OutOfBudget> {
        let mut best_score = i32::MIN;
        let mut best = Vec::new();
        for candidate in moves {
            let mut child = board.clone();
            apply_move(&mut child, candidate);
            // A window one wider than the best so far keeps ties, so they can be told apart by
            // lot.
            let floor = best_score.saturating_sub(1);
            let score = -self.negamax(
                &child,
                side.opponent(),
                depth.saturating_sub(1),
                CAPTURE_EXTENSION,
                i32::MIN.saturating_add(1),
                floor.saturating_neg(),
            )?;
            if score > best_score {
                best_score = score;
                best.clear();
            }
            if score == best_score {
                best.push(candidate);
            }
        }
        Ok(best)
    }

    fn negamax(
        &mut self,
        board: &Board,
        side: Side,
        depth: u8,
        extension: u8,
        mut alpha: i32,
        beta: i32,
    ) -> Result<i32, OutOfBudget> {
        if !self.budget.spend(1) {
            return Err(OutOfBudget);
        }
        let key = self.table.as_ref().map(|_| position_key(board, side));
        let remembered = key.and_then(|key| self.table.as_ref()?.probe(key));
        if let Some(entry) = remembered
            && entry.depth >= depth
            && entry.extension >= extension
        {
            match entry.bound {
                Bound::Exact => return Ok(entry.score),
                Bound::Lower if entry.score >= beta => return Ok(entry.score),
                Bound::Upper if entry.score <= alpha => return Ok(entry.score),
                Bound::Lower | Bound::Upper => {}
            }
        }

        let moves = legal_moves(board, side);
        if moves.is_empty() {
            // Closer losses are worse: the bot prefers to lose later and win sooner.
            return Ok(LOSS.saturating_add(i32::from(extension) + i32::from(depth)));
        }
        let capturing = moves
            .first()
            .is_some_and(|first| !first.captured.is_empty());
        let (remaining, remaining_extension) = match (depth, capturing) {
            (0, true) if extension > 0 => (1, extension - 1),
            (0, _) => return Ok(evaluate(board, side)),
            (depth, _) => (depth, extension),
        };
        let remembered_best = remembered.map(|entry| usize::from(entry.best_move));
        let mut ordered: Vec<(usize, &LegalMove)> = moves.iter().enumerate().collect();
        // The best move of an earlier search first, then bigger captures and crownings: they cut
        // the search soonest.
        ordered.sort_by_key(|(index, candidate)| {
            (
                Some(*index) != remembered_best,
                std::cmp::Reverse(candidate.captured.len() * 2 + usize::from(candidate.crowns)),
            )
        });
        let original_alpha = alpha;
        let mut best = i32::MIN.saturating_add(1);
        let mut best_move = 0;
        for (index, candidate) in ordered {
            let mut child = board.clone();
            apply_move(&mut child, candidate);
            let score = -self.negamax(
                &child,
                side.opponent(),
                remaining - 1,
                remaining_extension,
                beta.saturating_neg(),
                alpha.saturating_neg(),
            )?;
            if score > best {
                best = score;
                best_move = index;
            }
            alpha = alpha.max(score);
            if alpha >= beta {
                break;
            }
        }
        if let (Some(key), Some(table)) = (key, self.table.as_mut()) {
            let bound = if best <= original_alpha {
                Bound::Upper
            } else if best >= beta {
                Bound::Lower
            } else {
                Bound::Exact
            };
            table.store(TableEntry {
                key,
                score: best,
                bound,
                depth,
                extension,
                // Fewer than 256 legal moves in any position; a wrong guess only costs ordering.
                best_move: u8::try_from(best_move).unwrap_or(0),
            });
        }
        Ok(best)
    }
}
