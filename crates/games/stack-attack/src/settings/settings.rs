use mg_core::{Capped, GameSettings, SettingRange, SettingsError};

use crate::settings::RawSettings;

pub type FieldWidth = Capped<6, 32>;
/// At least 6 rows leave room for a worker (2 cells), a jump and the ceiling row.
pub type FieldHeight = Capped<6, 24>;

pub const MIN_SEATS: u8 = 1;
pub const MAX_SEATS: u8 = 4;

/// The legacy lobby had no settings (fixed 15x10 field); the size is configurable now and the
/// default keeps the legacy field. Valid by construction (see [`GameSettings::parse`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Settings {
    pub field_width: FieldWidth,
    pub field_height: FieldHeight,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            field_width: FieldWidth::constant::<15>(),
            field_height: FieldHeight::constant::<10>(),
        }
    }
}

impl GameSettings for Settings {
    type Raw = RawSettings;

    fn parse(raw: RawSettings) -> Result<Self, SettingsError> {
        Ok(Self {
            field_width: FieldWidth::new(raw.field_width)
                .map_err(SettingsError::at("field_width"))?,
            field_height: FieldHeight::new(raw.field_height)
                .map_err(SettingsError::at("field_height"))?,
        })
    }

    fn to_raw(&self) -> RawSettings {
        RawSettings {
            field_width: self.field_width.get(),
            field_height: self.field_height.get(),
        }
    }

    fn ranges(_raw: &RawSettings) -> Vec<SettingRange> {
        vec![
            FieldWidth::range("field_width"),
            FieldHeight::range("field_height"),
        ]
    }
}
