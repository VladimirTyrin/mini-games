use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Clone, Debug, clap::Parser)]
#[command(name = "mini_games_server", version, about = "Mini games server")]
pub struct Config {
    #[arg(long, env = "MG_LISTEN", default_value = "127.0.0.1:5000")]
    pub listen: SocketAddr,

    /// Built web client to serve at `/` (optional; production serves it from nginx).
    #[arg(long, env = "MG_STATIC_DIR")]
    pub static_dir: Option<PathBuf>,

    #[arg(long, env = "MG_ALLOW_GUESTS", default_value_t = true, action = clap::ArgAction::Set)]
    pub allow_guests: bool,

    /// OAuth Web client id; empty disables Google sign-in.
    #[arg(long, env = "MG_GOOGLE_CLIENT_ID", default_value = "")]
    pub google_client_id: String,

    /// Hex secret (>= 32 bytes) for session tokens. Without it a random one is generated and
    /// sessions do not survive restarts.
    #[arg(long, env = "MG_SESSION_SECRET", hide_env_values = true)]
    pub session_secret: Option<String>,

    #[arg(long, env = "MG_SESSION_DAYS", default_value_t = 30)]
    pub session_days: u64,

    /// `name:sha256hex,...` keys for network bots.
    #[arg(long, env = "MG_BOT_KEYS", default_value = "", hide_env_values = true)]
    pub bot_keys: String,

    /// Tokio worker threads (default: min(cores, 2)).
    #[arg(long, env = "MG_WORKER_THREADS")]
    pub worker_threads: Option<usize>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("MG_SESSION_SECRET must be at least 64 hex characters")]
    SessionSecret,
    #[error("MG_BOT_KEYS entry `{entry}`: {reason}")]
    BotKey { entry: String, reason: &'static str },
}

impl Config {
    pub fn session_secret_bytes(&self) -> Result<Option<Vec<u8>>, ConfigError> {
        let Some(hex) = self
            .session_secret
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            return Ok(None);
        };
        if hex.len() < 64 || hex.len() % 2 != 0 {
            return Err(ConfigError::SessionSecret);
        }
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                std::str::from_utf8(pair)
                    .ok()
                    .and_then(|text| u8::from_str_radix(text, 16).ok())
            })
            .collect::<Option<Vec<u8>>>()
            .map(Some)
            .ok_or(ConfigError::SessionSecret)
    }
}
