use crate::bot::Bot;
use crate::game::Game;
use crate::slot::Slot;

pub struct BotSeat<G: Game> {
    pub slot: Slot,
    pub bot: Box<dyn Bot<G>>,
    pub budget_nodes: u64,
}

impl<G: Game> std::fmt::Debug for BotSeat<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BotSeat")
            .field("slot", &self.slot)
            .field("bot", &self.bot)
            .finish()
    }
}
