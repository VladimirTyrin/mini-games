use mg_core::{Capped, GameSettings, SettingProblem, SettingRange, SettingsError};

use crate::settings::{RawSettings, Target};

pub type BoardSize = Capped<2, 10>;

/// Valid by construction (see [`GameSettings::parse`]): the target can be reached on the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub(crate) width: BoardSize,
    pub(crate) height: BoardSize,
    pub(crate) target: Target,
}

/// The largest tile a board can hold: every cell a different power of two, the last spawn a 4,
/// merging all the way down gives `2^(cells + 1)`.
fn largest_tile_on(width: BoardSize, height: BoardSize) -> u64 {
    let cells = width.get() * height.get();
    1_u64.checked_shl(cells + 1).unwrap_or(u64::MAX)
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
        let width = BoardSize::new(raw.width).map_err(SettingsError::at("width"))?;
        let height = BoardSize::new(raw.height).map_err(SettingsError::at("height"))?;
        let target = Target::new(raw.target).map_err(SettingsError::at("target"))?;
        if u64::from(target.get()) > largest_tile_on(width, height) {
            return Err(SettingsError::at("target")(SettingProblem::RuleBroken {
                rule: "cannot be reached on a board this small",
                value: target.get(),
            }));
        }
        Ok(Self {
            width,
            height,
            target,
        })
    }

    fn to_raw(&self) -> RawSettings {
        RawSettings {
            width: self.width.get(),
            height: self.height.get(),
            target: self.target.get(),
        }
    }

    fn ranges(raw: &RawSettings) -> Vec<SettingRange> {
        let mut target = Target::range("target");
        let largest_tile = largest_tile_on(
            BoardSize::saturating(raw.width),
            BoardSize::saturating(raw.height),
        );
        target.max = u32::try_from(largest_tile).map_or(target.max, |tile| target.max.min(tile));
        vec![
            BoardSize::range("width"),
            BoardSize::range("height"),
            target,
        ]
    }
}
