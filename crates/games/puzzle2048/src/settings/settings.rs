use mg_core::SettingRange;

/// Lobby settings as they arrive from clients and replay files (unvalidated); `Game::new`
/// rejects anything outside the limits below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub width: u32,
    pub height: u32,
    /// Winning tile value: a power of two in `MIN_TARGET..=MAX_TARGET`.
    pub target: u32,
}

impl Settings {
    pub const MIN_SIZE: u32 = 2;
    pub const MAX_SIZE: u32 = 10;
    pub const MIN_TARGET: u32 = 8;
    /// The legacy server had no upper bound (P3).
    pub const MAX_TARGET: u32 = 65_536;
    pub const WIDTH: SettingRange = SettingRange::new("width", Self::MIN_SIZE, Self::MAX_SIZE);
    pub const HEIGHT: SettingRange = SettingRange::new("height", Self::MIN_SIZE, Self::MAX_SIZE);
    /// Only the powers of two inside these bounds are valid (see [`crate::Target`]).
    pub const TARGET: SettingRange =
        SettingRange::new("target", Self::MIN_TARGET, Self::MAX_TARGET);

    /// Bounds of every numeric setting; the only list the UI reads (see
    /// [`mg_core::Game::setting_ranges`]).
    pub fn ranges(&self) -> Vec<SettingRange> {
        vec![Self::WIDTH, Self::HEIGHT, Self::TARGET]
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            width: 4,
            height: 4,
            target: 2048,
        }
    }
}
