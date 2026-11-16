use mg_core::{SettingRange, SetupError};

use crate::settings::HintMode;

pub const DEFAULT_REFILLS: u32 = 5;
/// Each refill can double the board, so this also caps it at `INITIAL_CELLS * 2^MAX_REFILLS`
/// cells.
pub const MAX_REFILLS: u32 = 10;
pub const LIMITED_INITIAL_HINTS: u32 = 3;
pub const LIMITED_HINTS_PER_REFILL: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub hint_mode: HintMode,
    pub refills: u32,
}

impl Default for Settings {
    /// Legacy rules, which are also the web client's defaults.
    fn default() -> Self {
        Self {
            hint_mode: HintMode::Limited,
            refills: DEFAULT_REFILLS,
        }
    }
}

impl Settings {
    pub const REFILLS: SettingRange = SettingRange::new("refills", 0, MAX_REFILLS);

    pub fn validate(&self) -> Result<(), SetupError> {
        Self::REFILLS.check(self.refills)
    }

    /// Bounds of every numeric setting; the only list the UI reads (see
    /// [`mg_core::Game::setting_ranges`]).
    pub fn ranges(&self) -> Vec<SettingRange> {
        vec![Self::REFILLS]
    }
}
