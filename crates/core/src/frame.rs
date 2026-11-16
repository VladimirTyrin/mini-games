//! Lockstep frames: the only thing that travels between the authority and clients during a match.

use crate::game::{Game, Slot};

/// One input inside a frame, applied in list order.
#[derive(Clone, Debug, PartialEq)]
pub enum Input<C, W> {
    /// A seat's command (human or bot).
    Command {
        /// Acting seat.
        slot: Slot,
        /// The command.
        cmd: C,
    },
    /// Randomness resolved by the authority.
    World(W),
    /// A seat left play for good.
    Dropped {
        /// The seat.
        slot: Slot,
    },
}

/// Everything that happened during one tick (or one turn).
#[derive(Clone, Debug, PartialEq)]
pub struct Frame<C, W> {
    /// Sequence number starting at 0, no gaps.
    pub tick: u64,
    /// Inputs in application order.
    pub inputs: Vec<Input<C, W>>,
    /// Authority's state hash after applying this frame (sent every N frames and on the last one).
    pub hash: Option<u64>,
}

/// Frame type for a game.
pub type GameFrame<G> = Frame<<G as Game>::Command, <G as Game>::WorldEvent>;
/// Input type for a game.
pub type GameInput<G> = Input<<G as Game>::Command, <G as Game>::WorldEvent>;

impl<C, W> Frame<C, W> {
    /// `true` if nothing happened besides time passing.
    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }
}
