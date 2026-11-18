use std::net::SocketAddr;
use std::path::PathBuf;

use crate::hex::decode_hex;
use crate::push::PushConfigError;
use crate::timing::Timing;

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

    /// WebSocket connections per client IP (an IPv6 client by its /64). Behind a reverse proxy
    /// on the same host the IP comes from `X-Real-IP` (trusted only when the peer is loopback).
    #[arg(long, env = "MG_MAX_CONNECTIONS_PER_IP", default_value_t = 32)]
    pub max_connections_per_ip: usize,

    /// SQLite database (friends, push targets); created and migrated at start. Without it they
    /// live in memory until the server stops.
    #[arg(long, env = "MG_DATABASE")]
    pub database: Option<PathBuf>,

    /// Firebase service account key file, base64: sends pushes to the Android app. Empty: off.
    #[arg(
        long,
        env = "MG_FCM_SERVICE_ACCOUNT",
        default_value = "",
        hide_env_values = true
    )]
    pub fcm_service_account: String,

    /// Hex P-256 private key (64 characters) signing Web Push messages. Empty: off.
    #[arg(
        long,
        env = "MG_VAPID_PRIVATE_KEY",
        default_value = "",
        hide_env_values = true
    )]
    pub vapid_private_key: String,

    /// Contact for push services about this sender: `https://<site>` or `mailto:<address>`.
    #[arg(long, env = "MG_VAPID_SUBJECT", default_value = "")]
    pub vapid_subject: String,

    #[arg(skip)]
    pub timing: Timing,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("MG_SESSION_SECRET must be at least 64 hex characters")]
    SessionSecret,
    #[error("MG_BOT_KEYS entry `{entry}`: {reason}")]
    BotKey { entry: String, reason: &'static str },
    #[error("push notifications: {0}")]
    Push(#[source] PushConfigError),
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
        if hex.len() < 64 {
            return Err(ConfigError::SessionSecret);
        }
        decode_hex(hex).map(Some).ok_or(ConfigError::SessionSecret)
    }
}
