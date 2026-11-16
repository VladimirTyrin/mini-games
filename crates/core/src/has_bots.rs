use crate::bot::Bot;
use crate::game::Game;

pub trait HasBots: Game {
    type BotKind: Copy + std::fmt::Debug + PartialEq + Eq + Send + Sync + 'static;

    /// In UI order.
    const BOT_KINDS: &'static [Self::BotKind];

    fn make_bot(kind: Self::BotKind) -> Box<dyn Bot<Self>>;

    /// Work budget per decision, see [`crate::Budget`].
    fn default_bot_budget(kind: Self::BotKind) -> u64;
}
