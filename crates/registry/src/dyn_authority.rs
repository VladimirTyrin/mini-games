use mg_core::{Awaiting, Clock, Slot, Status, Tick};
use mg_protocol::{WireError, proto};

use crate::dyn_bot_decision::DynBotDecision;
use crate::dyn_bot_turn::DynBotTurn;
use crate::game_kind::GameKind;
use crate::submit_error::SubmitError;

/// [`mg_core::Authority`] with the game type erased; speaks wire types.
pub trait DynAuthority: Send + std::fmt::Debug {
    fn kind(&self) -> GameKind;
    fn clock(&self) -> Clock;
    fn submit(&mut self, slot: Slot, command: &proto::GameCommand) -> Result<(), SubmitError>;
    // The secret methods default to a game that hides nothing; the registry's adapter forwards
    // them to the game.

    /// See [`mg_core::Authority::submit_secret`]: a private command, never relayed.
    fn submit_secret(
        &mut self,
        _slot: Slot,
        _command: &proto::GameSecretCommand,
    ) -> Result<(), SubmitError> {
        Err(SubmitError::Wire(WireError::WrongGame))
    }
    /// See [`mg_core::Authority::submit_default_secret`].
    fn submit_default_secret(&mut self, _slot: Slot) -> Result<(), SubmitError> {
        Err(SubmitError::Wire(WireError::WrongGame))
    }
    /// Whether the game still awaits `slot`'s private command (see `mg_core::Game::awaits_secret`).
    fn awaits_secret(&self, _slot: Slot) -> bool {
        false
    }
    /// What `slot` may see of the game's secret, for that seat's connection only.
    fn seat_secret(&self, _slot: Slot) -> Option<proto::GameSeatSecret> {
        None
    }
    fn drop_player(&mut self, slot: Slot);
    fn run_bots(&mut self);
    fn is_waiting_for_bot(&self) -> bool;
    /// See [`mg_core::Authority::begin_bot_turn`].
    fn begin_bot_turn(&mut self) -> Option<Box<dyn DynBotTurn>>;
    /// See [`mg_core::Authority::finish_bot_turn`]; `decision` must come from this authority.
    fn finish_bot_turn(&mut self, decision: DynBotDecision);
    fn has_pending_inputs(&self) -> bool;
    fn seal_frame(&mut self) -> proto::TickFrame;
    fn awaiting(&self) -> Awaiting;
    fn status(&self) -> Status;
    fn next_tick(&self) -> Tick;
    /// Frames `0..next_tick` without the empty ones (see [`mg_core::CompactLog`]), kept in wire
    /// form as they are sealed so a catch-up or replay file costs a copy, not a re-encode.
    fn compact_log(&self) -> &[proto::TickFrame];
}
