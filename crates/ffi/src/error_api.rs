use crate::boundary::{guard, write_bytes};
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
