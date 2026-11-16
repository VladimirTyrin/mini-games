use mg_core::{Slot, Viewer};

use crate::ffi_error::FfiError;

/// A negative slot renders for a spectator.
pub(crate) fn viewer_from_slot(viewer_slot: i32) -> Result<Viewer, FfiError> {
    if viewer_slot < 0 {
        return Ok(Viewer::Spectator);
    }
    u8::try_from(viewer_slot)
        .map(|slot| Viewer::Player(Slot::new(slot)))
        .map_err(|_| FfiError::ViewerSlot(viewer_slot))
}
