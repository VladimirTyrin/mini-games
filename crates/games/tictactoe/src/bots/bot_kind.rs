use mg_core::{Bot, HasBots};

use super::minimax_bot::MinimaxBot;
use super::random_bot::RandomBot;
use crate::tictactoe::TicTacToe;

pub const RANDOM_BOT_BUDGET: u64 = 1;
/// Search nodes per minimax move, sized to stay well below 200 ms per move on 15×15 / 5 in a
/// native release build.
pub const MINIMAX_BOT_BUDGET: u64 = 30_000;

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
