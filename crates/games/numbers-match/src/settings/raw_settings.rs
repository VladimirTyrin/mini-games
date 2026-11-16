use mg_core::GameSettings;

use crate::settings::{HintMode, Settings};

/// [`Settings`] as clients and replay files send them, unchecked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawSettings {
    pub hint_mode: HintMode,
    pub refills: u32,
}

impl Default for RawSettings {
    fn default() -> Self {
        Settings::default().to_raw()
    }
}
