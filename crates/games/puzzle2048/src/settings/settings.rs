use mg_core::{Capped, GameSettings, SettingRange, SettingsError};

use crate::settings::{RawSettings, Target};

pub type BoardSize = Capped<2, 10>;

/// Valid by construction (see [`GameSettings::parse`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub width: BoardSize,
    pub height: BoardSize,
    pub target: Target,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            width: BoardSize::constant::<4>(),
            height: BoardSize::constant::<4>(),
            target: Target::constant::<2048>(),
        }
    }
}

impl GameSettings for Settings {
    type Raw = RawSettings;

    fn parse(raw: RawSettings) -> Result<Self, SettingsError> {
        Ok(Self {
            width: BoardSize::new(raw.width).map_err(SettingsError::at("width"))?,
            height: BoardSize::new(raw.height).map_err(SettingsError::at("height"))?,
            target: Target::new(raw.target).map_err(SettingsError::at("target"))?,
        })
    }

    fn to_raw(&self) -> RawSettings {
        RawSettings {
            width: self.width.get(),
            height: self.height.get(),
            target: self.target.get(),
        }
    }

    fn ranges(_raw: &RawSettings) -> Vec<SettingRange> {
        vec![
            BoardSize::range("width"),
            BoardSize::range("height"),
            Target::range("target"),
        ]
    }
}
