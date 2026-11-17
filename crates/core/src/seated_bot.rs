use crate::bot_mind::BotMind;
use crate::game::Game;
use crate::slot::Slot;

#[derive(Debug)]
pub(crate) struct SeatedBot<G: Game> {
    pub(crate) slot: Slot,
    /// `None` while the bot thinks away from the authority (see [`crate::BotTurn`]).
    pub(crate) mind: Option<BotMind<G>>,
    pub(crate) budget_nodes: u64,
}
