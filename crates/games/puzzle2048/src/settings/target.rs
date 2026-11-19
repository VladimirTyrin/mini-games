use mg_core::{Capped, SettingProblem, SettingRule};

use crate::model::TileValue;

/// The legacy server had no upper bound (P3).
type TargetRange = Capped<8, 65_536>;

/// Winning tile value: a power of two within its range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Target(TargetRange);

impl Target {
    pub fn new(value: u32) -> Result<Self, SettingProblem> {
        let value = TargetRange::new(value)?;
        if !value.get().is_power_of_two() {
            return Err(SettingProblem::RuleBroken {
                rule: SettingRule::PowerOfTwo,
                value: value.get(),
            });
        }
        Ok(Self(value))
    }

    /// A target known at compile time; an invalid `VALUE` fails the build.
    pub const fn constant<const VALUE: u32>() -> Self {
        const { assert!(VALUE.is_power_of_two(), "target must be a power of two") };
        Self(TargetRange::constant::<VALUE>())
    }

    pub fn get(self) -> u32 {
        self.0.get()
    }

    /// Published bounds; only the powers of two inside them are valid.
    pub fn range(field: &'static str) -> mg_core::SettingRange {
        TargetRange::range(field)
    }

    pub fn is_reached_by(self, tile: TileValue) -> bool {
        tile.get() >= self.get()
    }
}
