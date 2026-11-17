use mg_core::{GameSettings, SettingRange, SettingsError};

use crate::settings::{CaptureRule, RawSettings, WhitePlayer};

/// Russian draughts on a fixed board; who plays white and whether a capture must go on to the
/// end are chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Settings {
    pub white_player: WhitePlayer,
    pub capture_rule: CaptureRule,
}

/// Seats at the board.
pub const PLAYERS: u8 = 2;

impl GameSettings for Settings {
    type Raw = RawSettings;

    fn parse(raw: RawSettings) -> Result<Self, SettingsError> {
        Ok(Self {
            white_player: raw.white_player,
            capture_rule: raw.capture_rule,
        })
    }

    fn to_raw(&self) -> RawSettings {
        RawSettings {
            white_player: self.white_player,
            capture_rule: self.capture_rule,
        }
    }

    fn ranges(_raw: &RawSettings) -> Vec<SettingRange> {
        Vec::new()
    }
}
