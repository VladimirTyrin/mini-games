use mg_core::GameSettings;

use crate::settings::{FirstPlayerMode, Settings};

/// [`Settings`] as clients and replay files send them, unchecked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RawSettings {
    pub field_width: u32,
    pub field_height: u32,
    /// Marks in a row needed to win.
    pub win_count: u32,
    pub first_player: FirstPlayerMode,
}

impl Default for RawSettings {
    fn default() -> Self {
        Settings::default().to_raw()
    }
}
