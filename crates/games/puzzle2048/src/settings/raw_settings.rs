use mg_core::GameSettings;

use crate::settings::Settings;

/// [`Settings`] as clients and replay files send them, unchecked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawSettings {
    pub width: u32,
    pub height: u32,
    pub target: u32,
}

impl Default for RawSettings {
    fn default() -> Self {
        Settings::default().to_raw()
    }
}
