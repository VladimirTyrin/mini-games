use mg_core::{Bot, HasBots};

use crate::bots::{CarefulBot, EfficientBot, RandomBot};
use crate::game::Snake;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BotKind {
    /// Greedy step towards the nearest food.
    Efficient,
    /// Uniformly random safe direction.
    Random,
    /// Efficient, but keeps off cells another head may enter and out of pockets too small.
    Careful,
}

impl HasBots for Snake {
    type BotKind = BotKind;

    const BOT_KINDS: &'static [BotKind] = &[BotKind::Efficient, BotKind::Random, BotKind::Careful];

    fn make_bot(kind: BotKind) -> Box<dyn Bot<Self>> {
        match kind {
            BotKind::Efficient => Box::new(EfficientBot),
            BotKind::Random => Box::new(RandomBot),
            BotKind::Careful => Box::new(CarefulBot),
        }
    }

    /// One node per food item scanned and per direction checked (max food is 50); Careful also
    /// one per cell of the pocket it measures, up to its own length, for each of three steps.
    fn default_bot_budget(kind: BotKind) -> u64 {
        match kind {
            BotKind::Efficient => 64,
            BotKind::Random => 8,
            BotKind::Careful => 4_096,
        }
    }
}
