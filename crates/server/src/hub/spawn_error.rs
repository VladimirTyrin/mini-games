use mg_registry::CreateError;

#[derive(Debug, thiserror::Error)]
pub(super) enum SpawnError {
    #[error("lobby not found")]
    LobbyNotFound,
    #[error("{game} needs at least {min_players} players")]
    NotEnoughPlayers { game: &'static str, min_players: u8 },
    #[error("cannot create the match: {0}")]
    Create(#[source] CreateError),
}
