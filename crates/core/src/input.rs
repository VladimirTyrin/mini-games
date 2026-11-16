use crate::game::Game;
use crate::slot::Slot;

/// One input inside a frame. Inputs are applied in list order.
#[derive(Clone, Debug, PartialEq)]
pub enum Input<C, W> {
    Command { slot: Slot, command: C },
    World(W),
    Dropped { slot: Slot },
}

pub type GameInput<G> = Input<<G as Game>::Command, <G as Game>::WorldEvent>;
