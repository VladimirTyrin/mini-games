use mg_core::{GameSettings, SettingRange, SettingsError};

use crate::settings::RawSettings;

/// Classic rules only: a 10×10 sea and the classic fleet, so there is nothing to choose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Settings;

impl GameSettings for Settings {
    type Raw = RawSettings;

    fn parse(_raw: RawSettings) -> Result<Self, SettingsError> {
        Ok(Self)
    }

    fn to_raw(&self) -> RawSettings {
        RawSettings
    }

    fn ranges(_raw: &RawSettings) -> Vec<SettingRange> {
        Vec::new()
    }
}
