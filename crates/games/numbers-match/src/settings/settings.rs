use mg_core::{Capped, GameSettings, SettingRange, SettingsError};

use crate::settings::{HintMode, RawSettings};

/// Each refill can double the board, so the bound also caps it at `INITIAL_CELLS * 2^10` cells.
pub type Refills = Capped<0, 10>;

pub const LIMITED_INITIAL_HINTS: u32 = 3;
pub const LIMITED_HINTS_PER_REFILL: u32 = 1;

/// Valid by construction (see [`GameSettings::parse`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub hint_mode: HintMode,
    pub refills: Refills,
}

impl Default for Settings {
    /// Legacy rules, which are also the web client's defaults.
    fn default() -> Self {
        Self {
            hint_mode: HintMode::Limited,
            refills: Refills::constant::<5>(),
        }
    }
}

impl GameSettings for Settings {
    type Raw = RawSettings;

    fn parse(raw: RawSettings) -> Result<Self, SettingsError> {
        Ok(Self {
            hint_mode: raw.hint_mode,
            refills: Refills::new(raw.refills).map_err(SettingsError::at("refills"))?,
        })
    }

    fn to_raw(&self) -> RawSettings {
        RawSettings {
            hint_mode: self.hint_mode,
            refills: self.refills.get(),
        }
    }

    fn ranges(_raw: &RawSettings) -> Vec<SettingRange> {
        vec![Refills::range("refills")]
    }
}
