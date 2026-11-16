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
