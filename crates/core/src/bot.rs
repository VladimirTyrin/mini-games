//! Built-in bots.

use crate::game::{Game, Slot};
use crate::rng::CoreRng;

/// Work limit for one bot decision, counted in abstract "nodes" (positions evaluated, cells
/// scanned...). Using work instead of wall time keeps bots deterministic on every platform.
#[derive(Clone, Debug)]
pub struct Budget {
    remaining: u64,
}

impl Budget {
    /// Allows `nodes` units of work.
    pub fn new(nodes: u64) -> Self {
        Self { remaining: nodes }
    }

    /// Consumes `n` units; returns `false` once the budget is exhausted (bot should wrap up).
    pub fn spend(&mut self, n: u64) -> bool {
        if self.remaining >= n {
            self.remaining -= n;
            true
        } else {
            self.remaining = 0;
            false
        }
    }

    /// Units left.
    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    /// `true` when nothing is left.
    pub fn exhausted(&self) -> bool {
        self.remaining == 0
    }
}

/// A deterministic bot. Runs only on the authority.
pub trait Bot<G: Game>: Send + std::fmt::Debug {
    /// Picks a command for `me`, or `None` to do nothing this time. Must return a command the game
    /// accepts whenever it is the bot's turn in a turn-based game.
    fn decide(
        &mut self,
        game: &G,
        me: Slot,
        rng: &mut CoreRng,
        budget: &mut Budget,
    ) -> Option<G::Command>;
}

/// A game that ships built-in bots.
pub trait HasBots: Game {
    /// Bot flavour (e.g. Random, Minimax).
    type BotKind: Copy + std::fmt::Debug + PartialEq + Eq + Send + Sync + 'static;

    /// All kinds, in UI order.
    const BOT_KINDS: &'static [Self::BotKind];

    /// Instantiates a bot.
    fn make_bot(kind: Self::BotKind) -> Box<dyn Bot<Self>>;

    /// Default work budget per decision.
    fn default_bot_budget(kind: Self::BotKind) -> u64;
}
