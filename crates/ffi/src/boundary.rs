//! Everything that touches foreign pointers or could unwind into foreign code goes through here.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::buffer::MgBuffer;
use crate::ffi_error::FfiError;
use crate::last_error;
use crate::status::MgStatus;

/// Runs an exported function body: converts its error or panic into a status and the thread's
/// last error. `AssertUnwindSafe` is sound here because a panic is reported as
/// [`MgStatus::Internal`], after which the contract tells the caller to drop the handle.
pub(crate) fn guard(body: impl FnOnce() -> Result<(), FfiError>) -> MgStatus {
    let error = match catch_unwind(AssertUnwindSafe(body)) {
        Ok(Ok(())) => return MgStatus::Ok,
        Ok(Err(error)) => error,
        Err(payload) => FfiError::Panic(panic_message(payload.as_ref())),
    };
    let refusal = error
        .refusal()
        .map(prost::Message::encode_to_vec)
        .unwrap_or_default();
    last_error::set(error.to_string(), refusal);
    error.status()
}

/// For the `*_free` functions, which have no status to return: a panic is only recorded as the
/// thread's last error.
pub(crate) fn guard_release(body: impl FnOnce()) {
    if let Err(payload) = catch_unwind(AssertUnwindSafe(body)) {
        last_error::set(
            FfiError::Panic(panic_message(payload.as_ref())).to_string(),
            Vec::new(),
        );
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_owned()
    }
}

/// `(data, len)` from the caller as a slice.
pub(crate) unsafe fn input_bytes<'a>(
    data: *const u8,
    len: usize,
    name: &'static str,
) -> Result<&'a [u8], FfiError> {
    if len == 0 {
        return Ok(&[]);
    }
    if data.is_null() {
        return Err(FfiError::NullArgument(name));
    }
    if isize::try_from(len).is_err() {
        return Err(FfiError::TooLong(name));
    }
    // SAFETY: by contract `data` points to `len` readable bytes that outlive the call.
    Ok(unsafe { std::slice::from_raw_parts(data, len) })
}

pub(crate) unsafe fn handle_ref<'a, T>(
    handle: *const T,
    name: &'static str,
) -> Result<&'a T, FfiError> {
    // SAFETY: by contract a non-null handle is live and not used by another thread.
    unsafe { handle.as_ref() }.ok_or(FfiError::NullArgument(name))
}

pub(crate) unsafe fn handle_mut<'a, T>(
    handle: *mut T,
    name: &'static str,
) -> Result<&'a mut T, FfiError> {
    // SAFETY: as in `handle_ref`; exclusive use is part of the contract.
    unsafe { handle.as_mut() }.ok_or(FfiError::NullArgument(name))
}

/// Writes `value` to a caller-provided out-pointer without reading or dropping what was there.
pub(crate) unsafe fn write_out<T>(
    out: *mut T,
    value: T,
    name: &'static str,
) -> Result<(), FfiError> {
    if out.is_null() {
        return Err(FfiError::NullArgument(name));
    }
    // SAFETY: by contract a non-null out-pointer points to writable memory of type T.
    unsafe { out.write(value) };
    Ok(())
}

pub(crate) unsafe fn write_bytes(
    out: *mut MgBuffer,
    bytes: Vec<u8>,
    name: &'static str,
) -> Result<(), FfiError> {
    // Checked first so a null out-pointer does not leak the allocation.
    if out.is_null() {
        return Err(FfiError::NullArgument(name));
    }
    // SAFETY: checked non-null above; the rest is the `write_out` contract.
    unsafe { write_out(out, MgBuffer::from_vec(bytes), name) }
}

/// Moves a new handle to the caller.
pub(crate) unsafe fn write_handle<T>(
    out: *mut *mut T,
    handle: T,
    name: &'static str,
) -> Result<(), FfiError> {
    if out.is_null() {
        return Err(FfiError::NullArgument(name));
    }
    // SAFETY: checked non-null above; the rest is the `write_out` contract.
    unsafe { write_out(out, Box::into_raw(Box::new(handle)), name) }
}

/// Takes back a handle created by [`write_handle`] and drops it.
pub(crate) unsafe fn release_handle<T>(handle: *mut T) {
    if handle.is_null() {
        return;
    }
    guard_release(|| {
        // SAFETY: by contract the handle came from `write_handle` and is freed exactly once.
        drop(unsafe { Box::from_raw(handle) });
    });
}

pub(crate) fn decode<M: prost::Message + Default>(
    bytes: &[u8],
    what: &'static str,
) -> Result<M, FfiError> {
    M::decode(bytes).map_err(|source| FfiError::Decode { what, source })
}
