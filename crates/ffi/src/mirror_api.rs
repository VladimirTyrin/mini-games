use mg_core::{Status, Tick};
use mg_protocol::{outcome_to_wire, proto};
use mg_registry::{LoadedReplay, create_mirror_from_header, load_replay_file};
use prost::Message;

use crate::boundary::{
    decode, guard, handle_mut, handle_ref, input_bytes, release_handle, write_bytes, write_handle,
    write_out,
};
use crate::buffer::MgBuffer;
use crate::ffi_error::FfiError;
use crate::mirror_handle::MgMirror;
use crate::status::MgStatus;
use crate::viewer::viewer_from_slot;

/// `header`: encoded `MatchHeader` as received in `MatchStarted` or `MatchCatchUp`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_mirror_new(
    header: *const u8,
    header_len: usize,
    out_mirror: *mut *mut MgMirror,
) -> MgStatus {
    guard(|| {
        let bytes = unsafe { input_bytes(header, header_len, "header") }?;
        let header: proto::MatchHeader = decode(bytes, "header")?;
        let mirror = MgMirror {
            mirror: create_mirror_from_header(&header)?,
            replay_log: None,
        };
        unsafe { write_handle(out_mirror, mirror, "out_mirror") }
    })
}

/// `replay`: encoded `ReplayFile`, validated as a whole. The mirror starts at tick 0; step it
/// with `mg_mirror_advance_to`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_mirror_from_replay_file(
    replay: *const u8,
    replay_len: usize,
    out_mirror: *mut *mut MgMirror,
) -> MgStatus {
    guard(|| {
        let bytes = unsafe { input_bytes(replay, replay_len, "replay") }?;
        let LoadedReplay { mirror, log } = load_replay_file(bytes)?;
        let mirror = MgMirror {
            mirror,
            replay_log: Some(log),
        };
        unsafe { write_handle(out_mirror, mirror, "out_mirror") }
    })
}

/// Null is ignored.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_mirror_free(mirror: *mut MgMirror) {
    unsafe { release_handle(mirror) }
}

/// `frame`: encoded `TickFrame`. `MG_STATUS_DESYNC` means the mirror no longer matches the
/// authority: build a new one from a catch-up.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_mirror_apply_frame(
    mirror: *mut MgMirror,
    frame: *const u8,
    frame_len: usize,
) -> MgStatus {
    guard(|| {
        let mirror = unsafe { handle_mut(mirror, "mirror") }?;
        let bytes = unsafe { input_bytes(frame, frame_len, "frame") }?;
        let frame: proto::TickFrame = decode(bytes, "frame")?;
        Ok(mirror.mirror.apply_frame(&frame)?)
    })
}

/// `catch_up`: encoded `MatchCatchUp`. Its frames are validated as a whole, then the ticks from
/// the mirror's next tick on are applied; the header is not used (create the mirror from it).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_mirror_apply_catch_up(
    mirror: *mut MgMirror,
    catch_up: *const u8,
    catch_up_len: usize,
) -> MgStatus {
    guard(|| {
        let mirror = unsafe { handle_mut(mirror, "mirror") }?;
        let bytes = unsafe { input_bytes(catch_up, catch_up_len, "catch_up") }?;
        let catch_up: proto::MatchCatchUp = decode(bytes, "catch_up")?;
        Ok(mirror
            .mirror
            .catch_up(catch_up.frames, Tick::new(catch_up.next_tick))?)
    })
}

/// Applies the replay log up to `stop_tick` (exclusive, capped at its end).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_mirror_advance_to(mirror: *mut MgMirror, stop_tick: u64) -> MgStatus {
    guard(|| {
        let mirror = unsafe { handle_mut(mirror, "mirror") }?;
        let log = mirror.replay_log.as_ref().ok_or(FfiError::NoReplayLog)?;
        Ok(mirror.mirror.apply_compact_log(log, Tick::new(stop_tick))?)
    })
}

/// End tick of the replay log; 0 for mirrors not made from a replay file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_mirror_replay_end_tick(
    mirror: *const MgMirror,
    out_tick: *mut u64,
) -> MgStatus {
    guard(|| {
        let mirror = unsafe { handle_ref(mirror, "mirror") }?;
        let end = mirror.replay_log.as_ref().map_or(0, |log| log.end().get());
        unsafe { write_out(out_tick, end, "out_tick") }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_mirror_next_tick(
    mirror: *const MgMirror,
    out_tick: *mut u64,
) -> MgStatus {
    guard(|| {
        let tick = unsafe { handle_ref(mirror, "mirror") }?
            .mirror
            .next_tick()
            .get();
        unsafe { write_out(out_tick, tick, "out_tick") }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_mirror_is_finished(
    mirror: *const MgMirror,
    out_finished: *mut bool,
) -> MgStatus {
    guard(|| {
        let finished = unsafe { handle_ref(mirror, "mirror") }?
            .mirror
            .status()
            .is_finished();
        unsafe { write_out(out_finished, finished, "out_finished") }
    })
}

/// The hash a `TickFrame.state_hash` / `ReportDesync.local_hash` carries.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_mirror_state_hash(
    mirror: *const MgMirror,
    out_hash: *mut u64,
) -> MgStatus {
    guard(|| {
        let hash = unsafe { handle_ref(mirror, "mirror") }?.mirror.state_hash();
        unsafe { write_out(out_hash, hash, "out_hash") }
    })
}

/// Encoded `GameView`; a negative `viewer_slot` renders for a spectator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_mirror_view(
    mirror: *const MgMirror,
    viewer_slot: i32,
    out_view: *mut MgBuffer,
) -> MgStatus {
    guard(|| {
        let mirror = unsafe { handle_ref(mirror, "mirror") }?;
        let view = mirror.mirror.view(viewer_from_slot(viewer_slot)?);
        unsafe { write_bytes(out_view, view.encode_to_vec(), "out_view") }
    })
}

/// Encoded `MatchResult` once the game is over; an empty buffer while it runs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_mirror_result(
    mirror: *const MgMirror,
    out_result: *mut MgBuffer,
) -> MgStatus {
    guard(|| {
        let bytes = match unsafe { handle_ref(mirror, "mirror") }?.mirror.status() {
            Status::Finished(outcome) => outcome_to_wire(&outcome).encode_to_vec(),
            Status::InProgress => Vec::new(),
        };
        unsafe { write_bytes(out_result, bytes, "out_result") }
    })
}
