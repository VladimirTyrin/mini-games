use mg_core::{Capped, GameSettings, SettingRange, SettingsError};

use crate::settings::{Dimensions, FirstPlayerMode, RawSettings, WinLength};

pub type FieldSize = Capped<3, 20>;
pub const PLAYERS: u8 = 2;

const DEFAULT_SIDE: u32 = 15;
const DEFAULT_WIN_COUNT: u32 = 5;
const _: () = assert!(DEFAULT_WIN_COUNT <= DEFAULT_SIDE);

/// Valid by construction, including the rule between fields (the win length fits the board), so
/// the fields are reachable only through [`GameSettings::parse`] and [`GameSettings::to_raw`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Settings {
    dimensions: Dimensions,
    win_length: WinLength,
    first_player: FirstPlayerMode,
}

impl Settings {
    pub(crate) fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    pub(crate) fn win_length(&self) -> WinLength {
        self.win_length
    }

    pub(crate) fn first_player(&self) -> FirstPlayerMode {
        self.first_player
    }
}

impl Default for Settings {
    /// Web client defaults.
    fn default() -> Self {
        Self {
            dimensions: Dimensions::new(
                FieldSize::constant::<DEFAULT_SIDE>(),
                FieldSize::constant::<DEFAULT_SIDE>(),
            ),
            win_length: WinLength::constant::<DEFAULT_WIN_COUNT>(),
            first_player: FirstPlayerMode::Random,
        }
    }
}

impl GameSettings for Settings {
    type Raw = RawSettings;

    fn parse(raw: RawSettings) -> Result<Self, SettingsError> {
        let dimensions = Dimensions::new(
            FieldSize::new(raw.field_width).map_err(SettingsError::at("field_width"))?,
            FieldSize::new(raw.field_height).map_err(SettingsError::at("field_height"))?,
        );
        Ok(Self {
            dimensions,
            win_length: WinLength::new(raw.win_count, dimensions)
                .map_err(SettingsError::at(WinLength::FIELD))?,
            first_player: raw.first_player,
        })
    }

    fn to_raw(&self) -> RawSettings {
        RawSettings {
            field_width: self.dimensions.width(),
            field_height: self.dimensions.height(),
            win_count: self.win_length.get(),
            first_player: self.first_player,
        }
    }

    /// The `win_count` bound follows the board; a board outside its limits is brought inside them
    /// so the bound stays usable while the form is being edited.
    fn ranges(raw: &RawSettings) -> Vec<SettingRange> {
        let shorter_side = FieldSize::saturating(raw.field_width.min(raw.field_height));
        vec![
            FieldSize::range("field_width"),
            FieldSize::range("field_height"),
            WinLength::range(shorter_side),
        ]
    }
}
