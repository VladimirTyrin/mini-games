use crate::bot::Bot;
use crate::game::Game;
use crate::rng::CoreRng;
use crate::slot::Slot;

#[derive(Debug)]
pub(crate) struct SeatedBot<G: Game> {
    pub(crate) slot: Slot,
    pub(crate) bot: Box<dyn Bot<G>>,
    pub(crate) rng: CoreRng,
    pub(crate) budget_nodes: u64,
}
