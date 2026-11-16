use mg_core::{Game, GameInfo};
use mg_protocol::proto::game_settings;
use mg_protocol::{WireError, proto};

use crate::games::games;

macro_rules! define_game_kind {
    ($($kind:ident => $game:path, $settings_case:ident, $bots:ident;)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum GameKind {
            $($kind,)*
        }

        impl GameKind {
            pub const ALL: &'static [GameKind] = &[$(GameKind::$kind,)*];

            pub fn of_settings(settings: &proto::GameSettings) -> Result<Self, WireError> {
                match settings.game.as_ref().ok_or(WireError::Missing("settings.game"))? {
                    $(game_settings::Game::$settings_case(_) => Ok(Self::$kind),)*
                }
            }

            pub fn info(self) -> GameInfo {
                match self {
                    $(Self::$kind => <$game as Game>::INFO,)*
                }
            }
        }
    };
}

games!(define_game_kind);
