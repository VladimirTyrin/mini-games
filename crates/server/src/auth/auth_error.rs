#[derive(Debug, thiserror::Error)]
pub(crate) enum AuthError {
    #[error("invalid display name: {0}")]
    InvalidDisplayName(&'static str),
    #[error("guests are not allowed on this server")]
    GuestsDisabled,
    #[error("Google sign-in is not configured on this server")]
    GoogleDisabled,
    #[error("invalid Google ID token: {0}")]
    InvalidGoogleToken(#[source] jsonwebtoken::errors::Error),
    #[error("Google ID token signed with unknown key")]
    UnknownGoogleKey,
    #[error("cannot fetch Google signing keys: {0}")]
    GoogleKeysUnavailable(String),
    #[error("invalid session token")]
    InvalidSessionToken,
    #[error("session token expired")]
    SessionExpired,
    #[error("unknown bot key")]
    UnknownBotKey,
}
