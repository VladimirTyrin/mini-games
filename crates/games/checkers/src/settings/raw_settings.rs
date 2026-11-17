use mg_core::GameSettings;

use crate::settings::{Settings, WhitePlayer};

/// [`Settings`] as clients and replay files send them, unchecked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawSettings {
    pub white_player: WhitePlayer,
}

impl Default for RawSettings {
    fn default() -> Self {
        Settings::default().to_raw()
    }
}
