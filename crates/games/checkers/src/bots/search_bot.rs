use mg_core::{Bot, Budget, CoreRng, Slot};

use super::evaluation::evaluate;
use crate::game::Checkers;
use crate::io::Command;
use crate::model::{Board, Side};
use crate::rules::{LegalMove, apply_move, legal_moves};

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
}

impl SearchBot {
    pub(crate) fn new(max_depth: u8) -> Self {
        Self { max_depth }
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
        let mut best: Vec<&LegalMove> = moves.iter().collect();
        for depth in 1..=self.max_depth {
            match root_search(game.board(), side, &moves, depth, budget) {
                Ok(found) => best = found,
                Err(OutOfBudget) => break,
            }
        }
        rng.pick(&best)
            .map(|chosen| Command::Move(chosen.path.clone()))
    }
}

/// The best moves at `depth`, all of equal score.
fn root_search<'moves>(
    board: &Board,
    side: Side,
    moves: &'moves [LegalMove],
    depth: u8,
    budget: &mut Budget,
) -> Result<Vec<&'moves LegalMove>, OutOfBudget> {
    let mut best_score = i32::MIN;
    let mut best = Vec::new();
    for candidate in moves {
        let mut child = board.clone();
        apply_move(&mut child, candidate);
        // A window one wider than the best so far keeps ties, so they can be told apart by lot.
        let floor = best_score.saturating_sub(1);
        let score = -negamax(
            &child,
            side.opponent(),
            depth.saturating_sub(1),
            CAPTURE_EXTENSION,
            i32::MIN.saturating_add(1),
            floor.saturating_neg(),
            budget,
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
    board: &Board,
    side: Side,
    depth: u8,
    extension: u8,
    mut alpha: i32,
    beta: i32,
    budget: &mut Budget,
) -> Result<i32, OutOfBudget> {
    if !budget.spend(1) {
        return Err(OutOfBudget);
    }
    let moves = legal_moves(board, side);
    if moves.is_empty() {
        // Closer losses are worse: the bot prefers to lose later and win sooner.
        return Ok(LOSS.saturating_add(i32::from(extension) + i32::from(depth)));
    }
    let capturing = moves
        .first()
        .is_some_and(|first| !first.captured.is_empty());
    let (depth, extension) = match (depth, capturing) {
        (0, true) if extension > 0 => (1, extension - 1),
        (0, _) => return Ok(evaluate(board, side)),
        (depth, _) => (depth, extension),
    };
    let mut ordered: Vec<&LegalMove> = moves.iter().collect();
    // Bigger captures and crownings first: they cut the search soonest.
    ordered.sort_by_key(|candidate| {
        std::cmp::Reverse(candidate.captured.len() * 2 + usize::from(candidate.crowns))
    });
    let mut best = i32::MIN.saturating_add(1);
    for candidate in ordered {
        let mut child = board.clone();
        apply_move(&mut child, candidate);
        let score = -negamax(
            &child,
            side.opponent(),
            depth - 1,
            extension,
            beta.saturating_neg(),
            alpha.saturating_neg(),
            budget,
        )?;
        best = best.max(score);
        alpha = alpha.max(score);
        if alpha >= beta {
            break;
        }
    }
    Ok(best)
}
