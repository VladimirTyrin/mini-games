use mg_core::{Bot, HasBots};

use super::{EasyBot, SearchBot};
use crate::game::Checkers;

pub const EASY_BOT_BUDGET: u64 = 1;
/// Positions the medium bot may visit per move: a few plies deep.
pub const MEDIUM_BOT_BUDGET: u64 = 20_000;
/// Positions the hard bot may visit per move: about a microsecond each in a native release
/// build (a quarter of a second for a move at worst), two to three times that in WASM on a phone.
pub const HARD_BOT_BUDGET: u64 = 250_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BotKind {
    Easy,
    Medium,
    Hard,
}

impl HasBots for Checkers {
    type BotKind = BotKind;

    const BOT_KINDS: &'static [BotKind] = &[BotKind::Easy, BotKind::Medium, BotKind::Hard];

    fn make_bot(kind: BotKind) -> Box<dyn Bot<Self>> {
        match kind {
            BotKind::Easy => Box::new(EasyBot),
            BotKind::Medium => Box::new(SearchBot::new(MEDIUM_MAX_DEPTH)),
            BotKind::Hard => Box::new(SearchBot::new(HARD_MAX_DEPTH)),
        }
    }

    fn default_bot_budget(kind: BotKind) -> u64 {
        match kind {
            BotKind::Easy => EASY_BOT_BUDGET,
            BotKind::Medium => MEDIUM_BOT_BUDGET,
            BotKind::Hard => HARD_BOT_BUDGET,
        }
    }
}

/// Plies the medium bot looks ahead at most: it sees simple shots but walks into deeper ones.
const MEDIUM_MAX_DEPTH: u8 = 3;
/// The hard bot deepens until its budget runs out.
const HARD_MAX_DEPTH: u8 = 16;
