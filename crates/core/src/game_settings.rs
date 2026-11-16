use std::fmt::Debug;

use crate::setting_range::SettingRange;
use crate::settings_error::SettingsError;

/// A game's settings, parsed: a value of the implementing type is valid by construction (ranges
/// are [`crate::Capped`] fields or types built on it), so [`crate::Game::new`] only checks what
/// depends on the players.
///
/// `Raw` is the unchecked form, field for field what clients, replay files and settings forms
/// carry. [`GameSettings::parse`] is the only way from one to the other.
pub trait GameSettings: Sized + Clone + Debug + Send + Sync + 'static {
    type Raw: Clone + Debug + PartialEq + Send + Sync + 'static;

    /// Checks every field and every rule between fields.
    fn parse(raw: Self::Raw) -> Result<Self, SettingsError>;

    fn to_raw(&self) -> Self::Raw;

    /// Bounds of every numeric setting as [`GameSettings::parse`] enforces them for `raw` (a
    /// bound may follow another field, e.g. a win length capped by the board), so a UI offers
    /// only valid values. `raw` may be invalid: it is what a form currently shows.
    fn ranges(raw: &Self::Raw) -> Vec<SettingRange>;
}
