use mg_core::GameSettings;

use crate::settings::{CaptureRule, Settings, WhitePlayer};

/// [`Settings`] as clients and replay files send them, unchecked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawSettings {
    pub white_player: WhitePlayer,
    pub capture_rule: CaptureRule,
}

impl Default for RawSettings {
    fn default() -> Self {
        Settings::default().to_raw()
    }
}
