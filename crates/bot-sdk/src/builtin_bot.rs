use mg_core::{Awaiting, Bot, Budget, CoreRng, HasBots, Slot};
use mg_protocol::{WireBots, WireError, proto};
use mg_registry::CreateError;

use crate::strategy::Strategy;
use crate::typed_mirror::TypedMirror;

/// Plays with one of the game's own built-in bots, as the server would, but over the network.
/// Useful as a baseline and as a sparring partner for new bots.
#[derive(Debug)]
pub struct BuiltinBot<G: WireBots> {
    bot: Box<dyn Bot<G>>,
    rng: CoreRng,
    budget_nodes: u64,
}

impl<G: WireBots> BuiltinBot<G> {
    /// `seed` feeds the bot's own randomness (tie-breaks, random moves).
    pub fn new(kind: G::BotKind, seed: u64) -> Self {
        Self {
            bot: G::make_bot(kind),
            rng: CoreRng::new(seed),
            budget_nodes: <G as HasBots>::default_bot_budget(kind),
        }
    }

    pub fn from_wire(kind: &proto::BotKind, seed: u64) -> Result<Self, WireError> {
        Ok(Self::new(G::bot_kind_from_wire(kind)?, seed))
    }
}

impl<G: WireBots> Strategy for BuiltinBot<G> {
    type Mirror = TypedMirror<G>;

    fn create_mirror(
        &mut self,
        header: &proto::MatchHeader,
    ) -> Result<TypedMirror<G>, CreateError> {
        TypedMirror::new(header)
    }

    fn decide(&mut self, mirror: &TypedMirror<G>, me: Slot) -> Option<proto::GameCommand> {
        let game = mirror.game();
        let may_act = match game.awaiting() {
            Awaiting::Turn(slot) => slot == me,
            Awaiting::Simultaneous => true,
            Awaiting::Nobody => false,
        };
        if !may_act {
            return None;
        }
        let mut budget = Budget::new(self.budget_nodes);
        self.bot
            .decide(game, me, &mut self.rng, &mut budget)
            .map(|command| G::command_to_wire(&command))
    }
}
