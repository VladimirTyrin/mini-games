use std::sync::Mutex;
use std::time::{Duration, Instant};

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;

use super::auth_error::AuthError;
use super::display_name::DisplayName;

const GOOGLE_ISSUERS: [&str; 2] = ["accounts.google.com", "https://accounts.google.com"];
/// Lower bound between refetches triggered by an unknown `kid`, so forged tokens cannot make us
/// hammer Google.
const MIN_REFETCH_INTERVAL: Duration = Duration::from_secs(60);

pub struct FetchedKeys {
    pub keys: JwkSet,
    pub max_age: Duration,
}

pub trait GoogleKeySource: Send + Sync {
    fn fetch(&self) -> Result<FetchedKeys, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleIdentity {
    pub subject: String,
    pub display_name: DisplayName,
}

#[derive(Deserialize)]
struct GoogleClaims {
    sub: String,
    name: Option<String>,
    given_name: Option<String>,
}

struct CachedKeys {
    keys: JwkSet,
    expires_at: Instant,
    fetched_at: Instant,
}

/// Verifies Google ID tokens. `verify` may block on HTTP when keys are stale: call it from a
/// blocking context.
pub struct GoogleVerifier {
    client_id: String,
    source: Box<dyn GoogleKeySource>,
    cache: Mutex<Option<CachedKeys>>,
}

impl std::fmt::Debug for GoogleVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoogleVerifier")
            .field("client_id", &self.client_id)
            .finish_non_exhaustive()
    }
}

impl GoogleVerifier {
    pub fn new(client_id: String, source: Box<dyn GoogleKeySource>) -> Self {
        Self {
            client_id,
            source,
            cache: Mutex::new(None),
        }
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn verify(&self, id_token: &str) -> Result<GoogleIdentity, AuthError> {
        let header = decode_header(id_token).map_err(AuthError::InvalidGoogleToken)?;
        let kid = header.kid.ok_or(AuthError::UnknownGoogleKey)?;
        let key = self.decoding_key(&kid)?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.client_id]);
        validation.set_issuer(&GOOGLE_ISSUERS);
        let claims = decode::<GoogleClaims>(id_token, &key, &validation)
            .map_err(AuthError::InvalidGoogleToken)?
            .claims;
        let raw_name = claims.name.or(claims.given_name).unwrap_or_default();
        Ok(GoogleIdentity {
            subject: claims.sub,
            display_name: DisplayName::sanitize(&raw_name, "Google player"),
        })
    }

    fn decoding_key(&self, kid: &str) -> Result<DecodingKey, AuthError> {
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = Instant::now();
        let needs_fetch = match cache.as_ref() {
            None => true,
            Some(cached) if cached.expires_at <= now => true,
            Some(cached) => {
                cached.keys.find(kid).is_none()
                    && now.duration_since(cached.fetched_at) >= MIN_REFETCH_INTERVAL
            }
        };
        if needs_fetch {
            match self.source.fetch() {
                Ok(fetched) => {
                    *cache = Some(CachedKeys {
                        keys: fetched.keys,
                        expires_at: now + fetched.max_age,
                        fetched_at: now,
                    });
                }
                // Stale keys are still better than failing every sign-in while Google is unreachable.
                Err(error) if cache.is_none() => {
                    return Err(AuthError::GoogleKeysUnavailable(error));
                }
                Err(error) => {
                    tracing::warn!(%error, "refreshing Google keys failed, using cached keys")
                }
            }
        }
        let jwk = cache
            .as_ref()
            .and_then(|cached| cached.keys.find(kid))
            .ok_or(AuthError::UnknownGoogleKey)?;
        DecodingKey::from_jwk(jwk).map_err(AuthError::InvalidGoogleToken)
    }
}

/// Fetches Google's JWKS over HTTPS.
#[derive(Debug)]
pub struct HttpGoogleKeySource {
    url: String,
}

impl HttpGoogleKeySource {
    pub const GOOGLE_CERTS_URL: &'static str = "https://www.googleapis.com/oauth2/v3/certs";

    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

impl GoogleKeySource for HttpGoogleKeySource {
    fn fetch(&self) -> Result<FetchedKeys, String> {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .build()
            .new_agent();
        let mut response = agent.get(&self.url).call().map_err(|e| e.to_string())?;
        let max_age = response
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok())
            .and_then(parse_max_age)
            .unwrap_or(Duration::from_secs(3600));
        let keys: JwkSet = response.body_mut().read_json().map_err(|e| e.to_string())?;
        Ok(FetchedKeys { keys, max_age })
    }
}

fn parse_max_age(cache_control: &str) -> Option<Duration> {
    cache_control
        .split(',')
        .filter_map(|directive| directive.trim().strip_prefix("max-age="))
        .find_map(|seconds| seconds.parse::<u64>().ok())
        .map(Duration::from_secs)
}

#[cfg(test)]
#[path = "google_tests.rs"]
mod tests;
