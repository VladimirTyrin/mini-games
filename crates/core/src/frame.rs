use crate::game::Game;
use crate::slot::Slot;
use crate::tick::Tick;

/// One input inside a frame. Inputs are applied in list order.
#[derive(Clone, Debug, PartialEq)]
pub enum Input<C, W> {
    Command { slot: Slot, command: C },
    World(W),
    Dropped { slot: Slot },
}

/// Everything that happened during one tick; the only payload exchanged during a match.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame<C, W> {
    pub tick: Tick,
    pub inputs: Vec<Input<C, W>>,
    /// Authority's state hash after this frame. Present every N frames and on the final frame.
    pub hash: Option<u64>,
}

pub type GameFrame<G> = Frame<<G as Game>::Command, <G as Game>::WorldEvent>;
pub type GameInput<G> = Input<<G as Game>::Command, <G as Game>::WorldEvent>;

impl<C, W> Frame<C, W> {
    pub fn empty(tick: Tick) -> Self {
        Self {
            tick,
            inputs: Vec::new(),
            hash: None,
        }
    }
}
