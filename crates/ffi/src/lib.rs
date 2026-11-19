//! C ABI over the game core, for bots and tools in other languages (the C# bots load it through
//! P/Invoke). Declarations: `include/mini_games.h`. Messages cross the boundary as binary
//! protobuf of `proto/minigames/v2`.
//!
//! Contract for every function (the header repeats it):
//! * The return value is an [`MgStatus`]; after a failure, [`mg_last_error_message`] describes
//!   it (per thread, kept until the next failure), and after [`MgStatus::Rejected`]
//!   [`mg_last_refusal`] gives the encoded `Refusal`.
//! * Input bytes are `(pointer, length)` pairs; the pointer may be null only when the length is 0.
//!   Out-pointers must point to writable memory of their type.
//! * Handles come from a `*_new`, `*_clone` or `*_from_*` function and are released exactly once
//!   with the matching `*_free`; a handle must not be used by two threads at the same time.
//! * An [`MgBuffer`] filled by the library belongs to the caller until [`mg_buffer_free`].
//! * Panics never unwind into the caller: they become [`MgStatus::Internal`], after which the
//!   handle involved should be freed rather than used.

// This crate is the FFI boundary: raw pointers from foreign code are its whole interface.
#![allow(unsafe_code)]
// One safety contract covers every exported function; it is documented above and in the header.
#![allow(clippy::missing_safety_doc)]

mod authority_api;
mod authority_step;
mod boundary;
mod buffer;
mod catalog_api;
mod error_api;
mod ffi_error;
mod last_error;
mod local_authority;
mod mirror_api;
mod mirror_handle;
mod status;
mod viewer;

pub use authority_api::{
    mg_authority_clone, mg_authority_drop_player, mg_authority_free,
    mg_authority_has_pending_inputs, mg_authority_is_finished, mg_authority_is_waiting_for_bot,
    mg_authority_new, mg_authority_next_tick, mg_authority_replay_file, mg_authority_result,
    mg_authority_run_bots, mg_authority_seal_frame, mg_authority_submit, mg_authority_tick_ms,
    mg_authority_view,
};
pub use buffer::{MgBuffer, mg_buffer_free};
pub use catalog_api::{mg_bot_kinds, mg_default_settings, mg_validate_settings};
pub use error_api::{
    MG_ABI_VERSION, mg_abi_version, mg_last_error_message, mg_last_refusal, mg_refusal_english,
};
pub use local_authority::MgAuthority;
pub use mirror_api::{
    mg_mirror_advance_to, mg_mirror_apply_catch_up, mg_mirror_apply_frame, mg_mirror_free,
    mg_mirror_from_replay_file, mg_mirror_is_finished, mg_mirror_new, mg_mirror_next_tick,
    mg_mirror_replay_end_tick, mg_mirror_result, mg_mirror_state_hash, mg_mirror_view,
};
pub use mirror_handle::MgMirror;
pub use status::MgStatus;
