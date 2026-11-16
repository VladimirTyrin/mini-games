use std::path::PathBuf;

use mg_registry::CreateError;

#[derive(Debug, thiserror::Error)]
pub enum ArenaError {
    #[error("unknown game `{id}`; known games: {}", known.join(", "))]
    UnknownGame {
        id: String,
        known: Vec<&'static str>,
    },
    #[error("{game} has no bot `{name}`; its bots: {}", known.join(", "))]
    UnknownBot {
        game: &'static str,
        name: String,
        known: Vec<String>,
    },
    #[error("settings are not valid protobuf JSON of GameSettings: {0}")]
    SettingsJson(#[source] serde_json::Error),
    #[error("the settings are for {settings}, not for {game}")]
    SettingsForAnotherGame {
        game: &'static str,
        settings: &'static str,
    },
    #[error("cannot set up the match: {0}")]
    Setup(#[from] CreateError),
    #[error("cannot write replay {}: {source}", path.display())]
    WriteReplay {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
