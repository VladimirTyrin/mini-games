use mg_core::{Clock, CompactLog, CompactLogError, HashPolicy, Status, Tick, Viewer};
use mg_protocol::proto;

use crate::apply_error::ApplyError;
use crate::game_kind::GameKind;

/// [`mg_core::Match`] with the game type erased; the receiving side of lockstep.
pub trait DynMirror: Send + std::fmt::Debug {
    fn kind(&self) -> GameKind;
    fn clock(&self) -> Clock;
    fn apply_frame(&mut self, frame: &proto::TickFrame) -> Result<(), ApplyError>;
    /// Applies ticks `next_tick..stop` (capped at the log end), filling omitted frames without
    /// allocating them; a replay steps through one log with a growing `stop`.
    fn apply_compact_log(
        &mut self,
        log: &CompactLog<proto::TickFrame>,
        stop: Tick,
    ) -> Result<(), ApplyError>;
    fn view(&self, viewer: Viewer) -> proto::GameView;
    fn status(&self) -> Status;
    fn next_tick(&self) -> Tick;
    fn state_hash(&self) -> u64;

    /// Validates an untrusted log (catch-up, replay file) of this game; see [`CompactLog`].
    fn compact_log(
        &self,
        frames: Vec<proto::TickFrame>,
        end: Tick,
    ) -> Result<CompactLog<proto::TickFrame>, CompactLogError> {
        CompactLog::new(frames, end, HashPolicy::for_clock(self.clock()))
    }

    /// Validates the whole log before applying any of it.
    fn catch_up(&mut self, frames: Vec<proto::TickFrame>, end: Tick) -> Result<(), ApplyError> {
        let log = self.compact_log(frames, end)?;
        self.apply_compact_log(&log, end)
    }
}
