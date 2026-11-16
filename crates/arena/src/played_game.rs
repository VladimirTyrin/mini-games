use mg_core::{Outcome, Tick};

#[derive(Debug)]
pub(crate) struct PlayedGame {
    pub(crate) length: Tick,
    /// `None` when the game hit the tick limit.
    pub(crate) outcome: Option<Outcome>,
    pub(crate) replay_written: bool,
}
