use mg_core::CancelToken;

use crate::dyn_bot_decision::DynBotDecision;

/// [`mg_core::BotTurn`] with the game type erased: thinks on its own copy of the game, so it can
/// run on another thread while the match goes on.
pub trait DynBotTurn: Send {
    fn think(self: Box<Self>, cancel: &CancelToken) -> DynBotDecision;
}
