use mg_protocol::proto;
use mg_registry::CreateError;

#[derive(Debug, thiserror::Error)]
pub(super) enum SpawnError {
    #[error("lobby not found")]
    LobbyNotFound,
    #[error("{game} needs at least {min_players} players")]
    NotEnoughPlayers { game: &'static str, min_players: u8 },
    #[error("cannot create the match: {0}")]
    Create(#[source] CreateError),
    #[error("the server restarts for an update in a moment: start the match again after it")]
    ServerUpdating,
}

impl SpawnError {
    pub(super) fn setting_problem(&self) -> Option<proto::SettingProblem> {
        match self {
            Self::Create(error) => error.setting_problem(),
            Self::LobbyNotFound | Self::NotEnoughPlayers { .. } | Self::ServerUpdating => None,
        }
    }

    pub(super) fn code(&self) -> proto::ErrorCode {
        match self {
            Self::ServerUpdating => proto::ErrorCode::ServerUpdating,
            Self::LobbyNotFound | Self::NotEnoughPlayers { .. } | Self::Create(_) => {
                proto::ErrorCode::InvalidSettings
            }
        }
    }
}
