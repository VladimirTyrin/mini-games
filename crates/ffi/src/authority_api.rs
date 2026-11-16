use mg_protocol::{proto, slot_from_wire};
use prost::Message;

use crate::boundary::{
    decode, guard, handle_mut, handle_ref, input_bytes, release_handle, write_bytes, write_handle,
    write_out,
};
use crate::buffer::MgBuffer;
use crate::local_authority::MgAuthority;
use crate::status::MgStatus;
use crate::viewer::viewer_from_slot;

/// `header`: encoded `MatchHeader` (settings and seats; a seat with `bot_kind` is played by that
/// built-in bot). The seed drives every random event and bot; keep it secret from players.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_authority_new(
    header: *const u8,
    header_len: usize,
    seed: u64,
    out_authority: *mut *mut MgAuthority,
) -> MgStatus {
    guard(|| {
        let bytes = unsafe { input_bytes(header, header_len, "header") }?;
        let authority = MgAuthority::new(decode(bytes, "header")?, seed)?;
        unsafe { write_handle(out_authority, authority, "out_authority") }
    })
}

/// An independent copy in the same state (for search); costs time proportional to the match so
/// far.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_authority_clone(
    authority: *const MgAuthority,
    out_authority: *mut *mut MgAuthority,
) -> MgStatus {
    guard(|| {
        let fork = unsafe { handle_ref(authority, "authority") }?.fork()?;
        unsafe { write_handle(out_authority, fork, "out_authority") }
    })
}

/// Null is ignored.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_authority_free(authority: *mut MgAuthority) {
    unsafe { release_handle(authority) }
}

/// `command`: encoded `GameCommand` of a human seat. Applied at once; `MG_STATUS_REJECTED` when
/// the game refuses it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_authority_submit(
    authority: *mut MgAuthority,
    slot: u32,
    command: *const u8,
    command_len: usize,
) -> MgStatus {
    guard(|| {
        let authority = unsafe { handle_mut(authority, "authority") }?;
        let bytes = unsafe { input_bytes(command, command_len, "command") }?;
        authority.submit(slot_from_wire(slot)?, decode(bytes, "command")?)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_authority_drop_player(
    authority: *mut MgAuthority,
    slot: u32,
) -> MgStatus {
    guard(|| {
        let authority = unsafe { handle_mut(authority, "authority") }?;
        authority.drop_player(slot_from_wire(slot)?);
        Ok(())
    })
}

/// Lets the built-in bots act (the one on turn, or all of them in a real-time game).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_authority_run_bots(authority: *mut MgAuthority) -> MgStatus {
    guard(|| {
        unsafe { handle_mut(authority, "authority") }?.run_bots();
        Ok(())
    })
}

/// Closes the current frame (after the commands of a turn-based move, or once per tick of a
/// real-time game) and returns it as an encoded `TickFrame`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_authority_seal_frame(
    authority: *mut MgAuthority,
    out_frame: *mut MgBuffer,
) -> MgStatus {
    guard(|| {
        let frame = unsafe { handle_mut(authority, "authority") }?.seal_frame()?;
        unsafe { write_bytes(out_frame, frame.encode_to_vec(), "out_frame") }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_authority_is_waiting_for_bot(
    authority: *const MgAuthority,
    out_waiting: *mut bool,
) -> MgStatus {
    guard(|| {
        let waiting = unsafe { handle_ref(authority, "authority") }?
            .authority()
            .is_waiting_for_bot();
        unsafe { write_out(out_waiting, waiting, "out_waiting") }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_authority_has_pending_inputs(
    authority: *const MgAuthority,
    out_pending: *mut bool,
) -> MgStatus {
    guard(|| {
        let pending = unsafe { handle_ref(authority, "authority") }?
            .authority()
            .has_pending_inputs();
        unsafe { write_out(out_pending, pending, "out_pending") }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_authority_is_finished(
    authority: *const MgAuthority,
    out_finished: *mut bool,
) -> MgStatus {
    guard(|| {
        let finished = unsafe { handle_ref(authority, "authority") }?
            .authority()
            .status()
            .is_finished();
        unsafe { write_out(out_finished, finished, "out_finished") }
    })
}

/// Milliseconds per frame of a real-time game; 0 for turn-based games.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_authority_tick_ms(
    authority: *const MgAuthority,
    out_tick_ms: *mut u32,
) -> MgStatus {
    guard(|| {
        let tick_ms = match unsafe { handle_ref(authority, "authority") }?
            .authority()
            .clock()
        {
            mg_core::Clock::Realtime { tick_ms } => tick_ms,
            mg_core::Clock::TurnBased => 0,
        };
        unsafe { write_out(out_tick_ms, tick_ms, "out_tick_ms") }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_authority_next_tick(
    authority: *const MgAuthority,
    out_tick: *mut u64,
) -> MgStatus {
    guard(|| {
        let tick = unsafe { handle_ref(authority, "authority") }?
            .authority()
            .next_tick()
            .get();
        unsafe { write_out(out_tick, tick, "out_tick") }
    })
}

/// Encoded `GameView` after the last sealed frame; a negative `viewer_slot` renders for a
/// spectator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_authority_view(
    authority: *const MgAuthority,
    viewer_slot: i32,
    out_view: *mut MgBuffer,
) -> MgStatus {
    guard(|| {
        let authority = unsafe { handle_ref(authority, "authority") }?;
        let view = authority.view(viewer_from_slot(viewer_slot)?);
        unsafe { write_bytes(out_view, view.encode_to_vec(), "out_view") }
    })
}

/// Encoded `MatchResult` once the game is over; an empty buffer while it runs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_authority_result(
    authority: *const MgAuthority,
    out_result: *mut MgBuffer,
) -> MgStatus {
    guard(|| {
        let result = unsafe { handle_ref(authority, "authority") }?.result();
        let bytes = result
            .as_ref()
            .map(Message::encode_to_vec)
            .unwrap_or_default();
        unsafe { write_bytes(out_result, bytes, "out_result") }
    })
}

/// Encoded `ReplayFile` of the match so far (complete once it is over).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_authority_replay_file(
    authority: *const MgAuthority,
    out_replay: *mut MgBuffer,
) -> MgStatus {
    guard(|| {
        let replay: proto::ReplayFile =
            unsafe { handle_ref(authority, "authority") }?.replay_file();
        unsafe { write_bytes(out_replay, replay.encode_to_vec(), "out_replay") }
    })
}
