use mg_core::{Bot, HasBots};

use crate::bots::{EfficientBot, RandomBot};
use crate::game::Snake;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BotKind {
    /// Greedy step towards the nearest food.
    Efficient,
    /// Uniformly random safe direction.
    Random,
}

impl HasBots for Snake {
    type BotKind = BotKind;

    const BOT_KINDS: &'static [BotKind] = &[BotKind::Efficient, BotKind::Random];

    fn make_bot(kind: BotKind) -> Box<dyn Bot<Self>> {
        match kind {
            BotKind::Efficient => Box::new(EfficientBot),
            BotKind::Random => Box::new(RandomBot),
        }
    }

    /// One node per food item scanned and per direction checked (max food is 50).
    fn default_bot_budget(kind: BotKind) -> u64 {
        match kind {
            BotKind::Efficient => 64,
            BotKind::Random => 8,
        }
    }
}
