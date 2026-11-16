use mg_core::{Bot, HasBots};

use super::minimax_bot::MinimaxBot;
use super::random_bot::RandomBot;
use crate::game::TicTacToe;

pub const RANDOM_BOT_BUDGET: u64 = 1;
/// Cells the minimax search may visit per move (probes, candidate scans, move updates). A cell
/// costs about 2.5–4 ns in a native release build, so a move stays near 100 ms at worst on any
/// allowed settings; on 15×15 / 5 the search reaches about as many nodes as the old 30 000-node
/// budget did.
pub const MINIMAX_BOT_BUDGET: u64 = 20_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BotKind {
    Minimax,
    Random,
}

impl HasBots for TicTacToe {
    type BotKind = BotKind;

    const BOT_KINDS: &'static [BotKind] = &[BotKind::Minimax, BotKind::Random];

    fn make_bot(kind: BotKind) -> Box<dyn Bot<Self>> {
        match kind {
            BotKind::Minimax => Box::new(MinimaxBot),
            BotKind::Random => Box::new(RandomBot),
        }
    }

    fn default_bot_budget(kind: BotKind) -> u64 {
        match kind {
            BotKind::Minimax => MINIMAX_BOT_BUDGET,
            BotKind::Random => RANDOM_BOT_BUDGET,
        }
    }
}
