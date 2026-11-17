use mg_core::{Bot, HasBots};

use crate::bots::HunterBot;
use crate::game::SeaBattle;

/// A shot is cheap at every level; the budget only bounds the work per decision.
pub const BOT_BUDGET: u64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BotKind {
    /// Fires at random and finishes a wounded ship only half the time.
    Easy,
    /// Finishes wounded ships along their line and hunts on a checkerboard.
    Medium,
    /// Fires where the ships still afloat fit most often, and hides its own fleet off the middle.
    Hard,
}

impl HasBots for SeaBattle {
    type BotKind = BotKind;

    const BOT_KINDS: &'static [BotKind] = &[BotKind::Easy, BotKind::Medium, BotKind::Hard];

    fn make_bot(kind: BotKind) -> Box<dyn Bot<Self>> {
        Box::new(HunterBot::new(kind))
    }

    fn default_bot_budget(_kind: BotKind) -> u64 {
        BOT_BUDGET
    }
}
