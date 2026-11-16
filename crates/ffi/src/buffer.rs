use std::mem::ManuallyDrop;

use crate::boundary::guard_release;

/// Bytes allocated by the library and owned by the caller until [`mg_buffer_free`]. `data` is
/// null when nothing was allocated (an empty result).
#[repr(C)]
#[derive(Debug)]
pub struct MgBuffer {
    pub data: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

impl MgBuffer {
    pub(crate) fn from_vec(bytes: Vec<u8>) -> Self {
        if bytes.capacity() == 0 {
            return Self::EMPTY;
        }
        let mut bytes = ManuallyDrop::new(bytes);
        Self {
            data: bytes.as_mut_ptr(),
            len: bytes.len(),
            capacity: bytes.capacity(),
        }
    }

    pub(crate) const EMPTY: Self = Self {
        data: std::ptr::null_mut(),
        len: 0,
        capacity: 0,
    };
}

/// Releases a buffer the library filled. Freeing an empty buffer is a no-op.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_buffer_free(buffer: MgBuffer) {
    if buffer.data.is_null() {
        return;
    }
    guard_release(|| {
        // SAFETY: by contract `buffer` came unchanged from `MgBuffer::from_vec` and is freed once,
        // so these are the parts of a live Vec<u8>.
        drop(unsafe { Vec::from_raw_parts(buffer.data, buffer.len, buffer.capacity) });
    });
}
