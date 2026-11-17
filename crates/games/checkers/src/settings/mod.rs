mod capture_rule;
mod raw_settings;
#[allow(clippy::module_inception)]
mod settings;
mod white_player;

pub use capture_rule::CaptureRule;
pub use raw_settings::RawSettings;
pub use settings::{PLAYERS, Settings};
pub use white_player::WhitePlayer;
