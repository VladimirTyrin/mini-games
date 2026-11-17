use mg_core::{Bot, Budget, CoreRng, Slot};

use crate::game::Checkers;
use crate::io::Command;

/// Half the time the move that takes the most pieces, otherwise any legal move.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct EasyBot;

impl Bot<Checkers> for EasyBot {
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
        budget.spend(1);
        let moves = game.legal_moves();
        let chosen = if rng.chance(1, 2) {
            let most = moves
                .iter()
                .map(|candidate| candidate.captured.len())
                .max()?;
            let greediest: Vec<_> = moves
                .iter()
                .filter(|candidate| candidate.captured.len() == most)
                .collect();
            rng.pick(&greediest).copied()?
        } else {
            rng.pick(&moves)?
        };
        Some(Command::Move(chosen.path.clone()))
    }
}
