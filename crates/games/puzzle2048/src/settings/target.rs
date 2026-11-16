use mg_core::SetupError;

use crate::model::TileValue;
use crate::settings::Settings;

/// Winning tile value, guaranteed to be a power of two within the [`Settings`] limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target(u32);

impl Target {
    pub fn new(value: u32) -> Result<Self, SetupError> {
        Settings::TARGET.check(value)?;
        if !value.is_power_of_two() {
            return Err(SetupError::setting(
                "target",
                format!("must be a power of two, got {value}"),
            ));
        }
        Ok(Self(value))
    }

    pub fn get(self) -> u32 {
        self.0
    }

    pub fn is_reached_by(self, tile: TileValue) -> bool {
        tile.get() >= self.0
    }
}
