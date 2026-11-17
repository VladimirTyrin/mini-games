use mg_core::{GameSettings, SettingRange, SettingsError};

use crate::settings::{RawSettings, WhitePlayer};

/// The board and the rules are fixed (Russian draughts); only who plays white is chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Settings {
    pub white_player: WhitePlayer,
}

/// Seats at the board.
pub const PLAYERS: u8 = 2;

impl GameSettings for Settings {
    type Raw = RawSettings;

    fn parse(raw: RawSettings) -> Result<Self, SettingsError> {
        Ok(Self {
            white_player: raw.white_player,
        })
    }

    fn to_raw(&self) -> RawSettings {
        RawSettings {
            white_player: self.white_player,
        }
    }

    fn ranges(_raw: &RawSettings) -> Vec<SettingRange> {
        Vec::new()
    }
}
