use std::num::NonZeroUsize;
use std::path::PathBuf;

use mg_core::Tick;
use mg_protocol::proto;

use crate::arena_bot::ArenaBot;

#[derive(Clone, Debug)]
pub struct ArenaConfig {
    pub settings: proto::GameSettings,
    /// One bot per seat, in slot order.
    pub bots: Vec<ArenaBot>,
    pub games: u32,
    pub seed: u64,
    pub threads: NonZeroUsize,
    /// A game still running after this many frames counts as unfinished (real-time games
    /// between strong bots may never end).
    pub max_ticks: Tick,
    /// Where to write replays of the games the first bot lost.
    pub replays: Option<PathBuf>,
}
