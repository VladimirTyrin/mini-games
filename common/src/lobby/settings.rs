use crate::{
    lobby_details, lobby_settings,
    NumbersMatchLobbySettings, Puzzle2048LobbySettings, SnakeLobbySettings,
    StackAttackLobbySettings, TicTacToeLobbySettings,
    validate_lobby_settings::ValidateLobbySettings,
};

#[derive(Debug, Clone)]
pub enum LobbySettings {
    Snake(SnakeLobbySettings),
    TicTacToe(TicTacToeLobbySettings),
    NumbersMatch(NumbersMatchLobbySettings),
    StackAttack(StackAttackLobbySettings),
    Puzzle2048(Puzzle2048LobbySettings),
}

impl LobbySettings {
    pub fn validate(&self, max_players: u32) -> Result<(), String> {
        match self {
            LobbySettings::Snake(s) => s.validate(max_players),
            LobbySettings::TicTacToe(t) => t.validate(max_players),
            LobbySettings::NumbersMatch(n) => n.validate(max_players),
            LobbySettings::StackAttack(s) => s.validate(max_players),
            LobbySettings::Puzzle2048(p) => p.validate(max_players),
        }
    }

    pub fn to_proto(&self) -> Option<lobby_details::Settings> {
        match self {
            LobbySettings::Snake(s) => Some(lobby_details::Settings::Snake(*s)),
            LobbySettings::TicTacToe(t) => Some(lobby_details::Settings::Tictactoe(*t)),
            LobbySettings::NumbersMatch(n) => Some(lobby_details::Settings::NumbersMatch(*n)),
            LobbySettings::StackAttack(s) => Some(lobby_details::Settings::StackAttack(*s)),
            LobbySettings::Puzzle2048(p) => Some(lobby_details::Settings::Puzzle2048(*p)),
        }
    }

    pub fn to_info_proto(&self) -> Option<crate::proto::game_service::LobbySettings> {
        Some(crate::proto::game_service::LobbySettings {
            settings: Some(match self {
                LobbySettings::Snake(s) => lobby_settings::Settings::Snake(*s),
                LobbySettings::TicTacToe(t) => lobby_settings::Settings::Tictactoe(*t),
                LobbySettings::NumbersMatch(n) => lobby_settings::Settings::NumbersMatch(*n),
                LobbySettings::StackAttack(s) => lobby_settings::Settings::StackAttack(*s),
                LobbySettings::Puzzle2048(p) => lobby_settings::Settings::Puzzle2048(*p),
            }),
        })
    }

    pub fn from_proto(settings: Option<lobby_settings::Settings>) -> Result<Self, String> {
        match settings {
            Some(lobby_settings::Settings::Snake(s)) => Ok(LobbySettings::Snake(s)),
            Some(lobby_settings::Settings::Tictactoe(t)) => Ok(LobbySettings::TicTacToe(t)),
            Some(lobby_settings::Settings::NumbersMatch(n)) => Ok(LobbySettings::NumbersMatch(n)),
            Some(lobby_settings::Settings::StackAttack(s)) => Ok(LobbySettings::StackAttack(s)),
            Some(lobby_settings::Settings::Puzzle2048(p)) => Ok(LobbySettings::Puzzle2048(p)),
            None => Err("No settings provided".to_string()),
        }
    }
}
