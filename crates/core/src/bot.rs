use crate::budget::Budget;
use crate::game::Game;
use crate::rng::CoreRng;
use crate::slot::Slot;

/// Built-in bot; runs only on the authority.
pub trait Bot<G: Game>: Send + std::fmt::Debug {
    /// In a turn-based game the bot must return a legal command on its turn, otherwise the
    /// authority drops it (forfeit) so the match cannot stall.
    fn decide(
        &mut self,
        game: &G,
        me: Slot,
        rng: &mut CoreRng,
        budget: &mut Budget,
    ) -> Option<G::Command>;
}
