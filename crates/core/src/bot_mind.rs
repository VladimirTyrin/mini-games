use crate::bot::Bot;
use crate::game::Game;
use crate::rng::CoreRng;

/// What a bot thinks with: the bot and its own random stream. Leaves its seat while a
/// [`crate::BotTurn`] is out.
#[derive(Debug)]
pub(crate) struct BotMind<G: Game> {
    pub(crate) bot: Box<dyn Bot<G>>,
    pub(crate) rng: CoreRng,
}
