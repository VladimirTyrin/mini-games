mod raw_settings;
#[allow(clippy::module_inception)]
mod settings;
mod white_player;

pub use raw_settings::RawSettings;
pub use settings::{PLAYERS, Settings};
pub use white_player::WhitePlayer;
