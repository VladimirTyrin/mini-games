use mg_core::{Bot, Budget, CoreRng, Slot};

use super::neighbourhood::Neighbourhood;
use super::search::Search;
use super::search_board::SearchBoard;
use super::tactics::tactical_move;
use crate::command::Command;
use crate::tictactoe::TicTacToe;

/// Legacy rules first ([`tactical_move`]), then an iterative-deepening alpha-beta search bounded
/// by the budget. Fully deterministic: it never uses the rng.
#[derive(Clone, Copy, Debug, Default)]
pub struct MinimaxBot;

impl Bot<TicTacToe> for MinimaxBot {
    fn decide(
        &mut self,
        game: &TicTacToe,
        me: Slot,
        _rng: &mut CoreRng,
        budget: &mut Budget,
    ) -> Option<Command> {
        let mark = game.mark_to_place(me)?;
        let mut board = SearchBoard::new(game, mark);
        let mut candidates = Vec::new();
        board.collect_candidates(Neighbourhood::WithinTwo, &mut candidates);
        for candidate in &mut candidates {
            candidate.probe = board.probe(candidate.point);
        }
        // The one-ply rules are cheap and bounded by the board size, so they always run; the
        // budget only bounds the search.
        let chosen = match tactical_move(&mut board, &candidates) {
            Some(candidate) => candidate,
            None => Search::new(board, budget).best_move(&candidates)?,
        };
        Some(Command::Place(chosen.point))
    }
}
