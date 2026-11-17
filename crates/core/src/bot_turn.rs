use crate::bot_mind::BotMind;
use crate::budget::Budget;
use crate::decided_bot_turn::DecidedBotTurn;
use crate::game::Game;
use crate::slot::Slot;

/// A turn-based bot's move worked out on a copy of the game, so the host stays free (to take a
/// player leaving, say) while a slow bot thinks. The result goes to
/// [`crate::Authority::finish_bot_turn`].
#[derive(Debug)]
pub struct BotTurn<G: Game> {
    pub(crate) slot: Slot,
    pub(crate) mind: BotMind<G>,
    pub(crate) budget_nodes: u64,
    pub(crate) game: G,
    pub(crate) revision: u64,
}

impl<G: Game> BotTurn<G> {
    pub fn think(mut self) -> DecidedBotTurn<G> {
        let mut budget = Budget::new(self.budget_nodes);
        let command = self
            .mind
            .bot
            .decide(&self.game, self.slot, &mut self.mind.rng, &mut budget);
        DecidedBotTurn {
            slot: self.slot,
            mind: self.mind,
            revision: self.revision,
            command,
        }
    }
}
