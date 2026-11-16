use mg_core::{Bot, HasBots};

use super::guessing_bot::GuessingBot;
use crate::game::WordHunt;

/// Candidates checked against the board per decision: a few thousand answers times up to 30
/// guesses fits easily; the budget only guards against surprises.
pub const BOT_BUDGET: u64 = 1_000_000;

/// How much of the board a bot takes into account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BotKind {
    /// Forgetful: every hint, but of the last two guesses only.
    Easy,
    /// Greens and yellows of every guess; letters ruled out (grey) are ignored.
    Medium,
    /// Every hint, letters ruled out (grey) included.
    Hard,
}

impl HasBots for WordHunt {
    type BotKind = BotKind;

    const BOT_KINDS: &'static [BotKind] = &[BotKind::Medium, BotKind::Easy, BotKind::Hard];

    fn make_bot(kind: BotKind) -> Box<dyn Bot<Self>> {
        Box::new(GuessingBot::new(kind))
    }

    fn default_bot_budget(_kind: BotKind) -> u64 {
        BOT_BUDGET
    }
}
