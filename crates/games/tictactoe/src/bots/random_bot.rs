use mg_core::{Bot, Budget, CoreRng, Slot};

use super::neighbourhood::Neighbourhood;
use super::search_board::SearchBoard;
use crate::command::Command;
use crate::tictactoe::TicTacToe;

/// Uniform over empty cells within two cells (Chebyshev) of any mark; the centre on an empty
/// board.
#[derive(Clone, Copy, Debug, Default)]
pub struct RandomBot;

impl Bot<TicTacToe> for RandomBot {
    fn decide(
        &mut self,
        game: &TicTacToe,
        me: Slot,
        rng: &mut CoreRng,
        budget: &mut Budget,
    ) -> Option<Command> {
        let mark = game.mark_to_place(me)?;
        budget.spend(1);
        let mut candidates = Vec::new();
        SearchBoard::new(game, mark).collect_candidates(Neighbourhood::WithinTwo, &mut candidates);
        let chosen = rng.pick(&candidates)?;
        Some(Command::Place(chosen.point))
    }
}
