use mg_core::{BotTurn, CancelToken, Game};

use crate::dyn_bot_decision::DynBotDecision;
use crate::dyn_bot_turn::DynBotTurn;

#[derive(Debug)]
pub(crate) struct TypedBotTurn<G: Game>(pub(crate) BotTurn<G>);

impl<G: Game> DynBotTurn for TypedBotTurn<G> {
    fn think(self: Box<Self>, cancel: &CancelToken) -> DynBotDecision {
        DynBotDecision::new(self.0.think(cancel))
    }
}
