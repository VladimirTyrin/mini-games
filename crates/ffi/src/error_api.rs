use mg_protocol::{proto, refusal_english};

use crate::boundary::{decode, guard, input_bytes, write_bytes};
use crate::buffer::MgBuffer;
use crate::last_error;
use crate::status::MgStatus;

/// Bumped on any incompatible change to the exported functions or types.
pub const MG_ABI_VERSION: u32 = 1;

/// Lets a caller check that it loaded the library it was written for.
#[unsafe(no_mangle)]
pub extern "C" fn mg_abi_version() -> u32 {
    MG_ABI_VERSION
}

/// UTF-8 description of the last failure on this thread; empty when nothing failed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_last_error_message(out_message: *mut MgBuffer) -> MgStatus {
    guard(|| unsafe { write_bytes(out_message, last_error::get(), "out_message") })
}

/// Encoded `Refusal` of the last failure on this thread when it was `MG_STATUS_REJECTED`;
/// empty otherwise. The same value a server sends in `ErrorResponse.refusal`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_last_refusal(out_refusal: *mut MgBuffer) -> MgStatus {
    guard(|| unsafe { write_bytes(out_refusal, last_error::refusal(), "out_refusal") })
}

/// UTF-8 English of an encoded `Refusal` (from `mg_last_refusal` or a server's
/// `ErrorResponse.refusal`), for logs: the text a server puts in `ErrorResponse.message`, which
/// names the detail of malformed input instead.
/// A reason this library does not know reads as a generic refusal.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_refusal_english(
    refusal: *const u8,
    refusal_len: usize,
    out_text: *mut MgBuffer,
) -> MgStatus {
    guard(|| {
        let bytes = unsafe { input_bytes(refusal, refusal_len, "refusal") }?;
        let refusal: proto::Refusal = decode(bytes, "refusal")?;
        let english = refusal_english(&refusal).as_bytes().to_vec();
        unsafe { write_bytes(out_text, english, "out_text") }
    })
}
