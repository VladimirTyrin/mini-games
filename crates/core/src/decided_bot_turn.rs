use crate::bot_mind::BotMind;
use crate::game::Game;
use crate::slot::Slot;

/// The outcome of [`crate::BotTurn::think`].
#[derive(Debug)]
pub struct DecidedBotTurn<G: Game> {
    pub(crate) slot: Slot,
    pub(crate) mind: BotMind<G>,
    /// The authority's revision the bot thought about; a move for an older one is stale.
    pub(crate) revision: u64,
    pub(crate) command: Option<G::Command>,
}
