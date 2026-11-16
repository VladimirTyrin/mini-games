use mg_core::{Awaiting, Clock, Slot, Status, Tick};
use mg_protocol::proto;

use crate::game_kind::GameKind;
use crate::submit_error::SubmitError;

/// [`mg_core::Authority`] with the game type erased; speaks wire types.
pub trait DynAuthority: Send + std::fmt::Debug {
    fn kind(&self) -> GameKind;
    fn clock(&self) -> Clock;
    fn submit(&mut self, slot: Slot, command: &proto::GameCommand) -> Result<(), SubmitError>;
    fn drop_player(&mut self, slot: Slot);
    fn run_bots(&mut self);
    fn is_waiting_for_bot(&self) -> bool;
    fn has_pending_inputs(&self) -> bool;
    fn seal_frame(&mut self) -> proto::TickFrame;
    fn awaiting(&self) -> Awaiting;
    fn status(&self) -> Status;
    fn next_tick(&self) -> Tick;
    /// Frames `0..next_tick` without the empty ones (see [`mg_core::CompactLog`]), kept in wire
    /// form as they are sealed so a catch-up or replay file costs a copy, not a re-encode.
    fn compact_log(&self) -> &[proto::TickFrame];
}
