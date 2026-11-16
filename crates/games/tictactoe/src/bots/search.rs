use mg_core::Budget;

use super::candidate::Candidate;
use super::neighbourhood::Neighbourhood;
use super::root_move::RootMove;
use super::score::Score;
use super::search_board::SearchBoard;
use super::side::Side;

/// Moves searched at the root and at inner nodes, best-ordered first. Forced moves (wins,
/// blocks of an immediate loss) are always ordered or filtered to the front, so the beam only
/// drops quiet moves.
const ROOT_BEAM: usize = 16;
const INNER_BEAM: usize = 8;
const MAX_DEPTH: usize = 32;

/// Iterative-deepening alpha-beta over [`SearchBoard`]. The budget is charged with the board's
/// work (cells visited), including the root probes made before the search started.
#[derive(Debug)]
pub(crate) struct Search<'a> {
    board: &'a mut SearchBoard,
    budget: &'a mut Budget,
    charged_work: u64,
    budget_exhausted: bool,
    /// Candidate buffers per ply, reused across nodes to keep the search allocation-free.
    buffers: Vec<Vec<Candidate>>,
}

impl<'a> Search<'a> {
    pub(crate) fn new(board: &'a mut SearchBoard, budget: &'a mut Budget) -> Self {
        Self {
            board,
            budget,
            charged_work: 0,
            budget_exhausted: false,
            buffers: Vec::new(),
        }
    }

    /// `root` must carry fresh probes. Returns the best move of the deepest completed iteration,
    /// or the best-ordered move when the budget does not even cover the probes made so far;
    /// `None` only for an empty `root`.
    pub(crate) fn best_move(mut self, root: &[Candidate]) -> Option<Candidate> {
        let mut moves: Vec<RootMove> = root
            .iter()
            .map(|&candidate| RootMove {
                candidate,
                score: Score::MIN,
            })
            .collect();
        moves.sort_by_key(|root_move| {
            std::cmp::Reverse(root_move.candidate.probe.priority(Side::Bot))
        });
        moves.truncate(ROOT_BEAM);
        let mut best = moves.first()?.candidate;
        if moves.len() == 1 || !self.charge_work() {
            return Some(best);
        }
        let max_depth = self.board.empty_cells().min(MAX_DEPTH);
        for depth in 1..=max_depth {
            if !self.score_root(&mut moves, depth) {
                break;
            }
            // Stable: ties keep the previous iteration's order, which keeps the result
            // deterministic and lets the next iteration search the principal move first.
            moves.sort_by_key(|root_move| std::cmp::Reverse(root_move.score));
            let Some(leader) = moves.first() else {
                break;
            };
            best = leader.candidate;
            if leader.score.is_decisive() {
                break;
            }
        }
        Some(best)
    }

    /// Returns `false` if the budget ran out; the scores are then incomplete and must be ignored.
    fn score_root(&mut self, moves: &mut [RootMove], depth: usize) -> bool {
        let mut alpha = Score::MIN;
        for root_move in moves.iter_mut() {
            let candidate = root_move.candidate;
            root_move.score = if depth == 1 {
                Score::heuristic(
                    self.board
                        .evaluation()
                        .saturating_add(candidate.probe.evaluation_delta(Side::Bot)),
                )
            } else {
                self.board.play(&candidate, Side::Bot);
                let score = self.node(depth - 1, 2, Side::Opponent, alpha, Score::MAX);
                self.board.undo(&candidate, Side::Bot);
                score
            };
            if self.budget_exhausted {
                return false;
            }
            alpha = alpha.max(root_move.score);
        }
        true
    }

    /// Value of the position with `side` to move; `ply` is the number of the move about to be
    /// made, counting the root move as 1.
    fn node(
        &mut self,
        depth: usize,
        ply: u32,
        side: Side,
        mut alpha: Score,
        mut beta: Score,
    ) -> Score {
        let mut moves = self.take_buffer(ply);
        self.board
            .collect_candidates(Neighbourhood::Adjacent, &mut moves);
        if moves.is_empty() {
            self.return_buffer(ply, moves);
            return Score::DRAW;
        }
        for candidate in moves.iter_mut() {
            candidate.probe = self.board.probe(candidate.point);
        }
        if !self.charge_work() {
            self.return_buffer(ply, moves);
            return Score::DRAW;
        }
        if moves
            .iter()
            .any(|candidate| candidate.probe.of(side).completes_line)
        {
            self.return_buffer(ply, moves);
            return Score::win_for(side, ply);
        }
        // Anything but blocking an immediate opponent win loses at once, so only blocks count.
        if moves
            .iter()
            .any(|candidate| candidate.probe.of(side.other()).completes_line)
        {
            moves.retain(|candidate| candidate.probe.of(side.other()).completes_line);
        }
        moves.sort_by_key(|candidate| std::cmp::Reverse(candidate.probe.priority(side)));
        moves.truncate(INNER_BEAM);

        let mut best = match side {
            Side::Bot => Score::MIN,
            Side::Opponent => Score::MAX,
        };
        for candidate in &moves {
            if self.budget_exhausted {
                break;
            }
            let score = if depth == 1 {
                Score::heuristic(
                    self.board
                        .evaluation()
                        .saturating_add(candidate.probe.evaluation_delta(side)),
                )
            } else {
                self.board.play(candidate, side);
                let score = self.node(depth - 1, ply + 1, side.other(), alpha, beta);
                self.board.undo(candidate, side);
                score
            };
            match side {
                Side::Bot => {
                    best = best.max(score);
                    alpha = alpha.max(score);
                }
                Side::Opponent => {
                    best = best.min(score);
                    beta = beta.min(score);
                }
            }
            if alpha >= beta {
                break;
            }
        }
        self.return_buffer(ply, moves);
        best
    }

    /// Spends the board work done since the last call; `false` once the budget is exhausted,
    /// after which every score is incomplete and must be ignored.
    fn charge_work(&mut self) -> bool {
        let work_done = self.board.work_done();
        let work = work_done.saturating_sub(self.charged_work);
        self.charged_work = work_done;
        if !self.budget.spend(work) {
            self.budget_exhausted = true;
        }
        !self.budget_exhausted
    }

    fn take_buffer(&mut self, ply: u32) -> Vec<Candidate> {
        self.buffers
            .get_mut(ply as usize)
            .map(std::mem::take)
            .unwrap_or_default()
    }

    fn return_buffer(&mut self, ply: u32, buffer: Vec<Candidate>) {
        let ply = ply as usize;
        if self.buffers.len() <= ply {
            self.buffers.resize_with(ply + 1, Vec::new);
        }
        if let Some(slot) = self.buffers.get_mut(ply) {
            *slot = buffer;
        }
    }
}
