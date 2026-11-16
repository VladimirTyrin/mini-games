use crate::game::Game;
use crate::input::Input;
use crate::tick::Tick;

/// Everything that happened during one tick; the only payload exchanged during a match.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame<C, W> {
    pub tick: Tick,
    pub inputs: Vec<Input<C, W>>,
    /// Authority's state hash after this frame. Present every N frames and on the final frame.
    pub hash: Option<u64>,
}

pub type GameFrame<G> = Frame<<G as Game>::Command, <G as Game>::WorldEvent>;

impl<C, W> Frame<C, W> {
    pub fn empty(tick: Tick) -> Self {
        Self {
            tick,
            inputs: Vec::new(),
            hash: None,
        }
    }

    /// Compact logs omit frames without inputs and without a hash; receivers recreate them.
    pub fn belongs_in_compact_log(&self) -> bool {
        !self.inputs.is_empty() || self.hash.is_some()
    }
}
