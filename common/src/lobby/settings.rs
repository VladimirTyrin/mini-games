use crate::{
    lobby_details, lobby_settings,
    SnakeLobbySettings, TicTacToeLobbySettings,
    validate_lobby_settings::ValidateLobbySettings,
};

#[derive(Debug, Clone)]
pub enum LobbySettings {
    Snake(SnakeLobbySettings),
    TicTacToe(TicTacToeLobbySettings),
}

impl LobbySettings {
    pub fn validate(&self, max_players: u32) -> Result<(), String> {
        match self {
            LobbySettings::Snake(s) => s.validate(max_players),
            LobbySettings::TicTacToe(t) => t.validate(max_players),
        }
    }

    pub fn to_proto(&self) -> Option<lobby_details::Settings> {
        match self {
            LobbySettings::Snake(s) => Some(lobby_details::Settings::Snake(*s)),
            LobbySettings::TicTacToe(t) => Some(lobby_details::Settings::Tictactoe(*t)),
        }
    }

    pub fn to_info_proto(&self) -> Option<crate::proto::game_service::LobbySettings> {
        Some(crate::proto::game_service::LobbySettings {
            settings: Some(match self {
                LobbySettings::Snake(s) => lobby_settings::Settings::Snake(*s),
                LobbySettings::TicTacToe(t) => lobby_settings::Settings::Tictactoe(*t),
            }),
        })
    }

    pub fn from_proto(settings: Option<lobby_settings::Settings>) -> Result<Self, String> {
        match settings {
            Some(lobby_settings::Settings::Snake(s)) => Ok(LobbySettings::Snake(s)),
            Some(lobby_settings::Settings::Tictactoe(t)) => Ok(LobbySettings::TicTacToe(t)),
            None => Err("No settings provided".to_string()),
        }
    }
}
