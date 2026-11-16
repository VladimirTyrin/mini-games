use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;
use tokio::time::Instant;

use super::auth_error::AuthError;
use super::display_name::DisplayName;
use super::key_fetch_error::KeyFetchError;

const GOOGLE_ISSUERS: [&str; 2] = ["accounts.google.com", "https://accounts.google.com"];
/// Lower bound between refetches triggered by an unknown `kid` or expired keys, so forged
/// tokens cannot make us hammer Google.
const MIN_REFETCH_INTERVAL: Duration = Duration::from_secs(60);
/// Without any keys every sign-in needs a fetch; after a failure they fail fast for this long.
const EMPTY_CACHE_RETRY_INTERVAL: Duration = Duration::from_secs(5);
/// How long a verification waits for another thread's fetch when there are no keys at all.
const FETCH_WAIT: Duration = Duration::from_secs(15);

pub(crate) struct FetchedKeys {
    pub keys: JwkSet,
    pub max_age: Duration,
}

pub(crate) trait GoogleKeySource: Send + Sync {
    fn fetch(&self) -> Result<FetchedKeys, KeyFetchError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GoogleIdentity {
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
}

#[derive(Default)]
struct KeyCache {
    keys: Option<CachedKeys>,
    /// Last fetch attempt, successful or not: the base for every backoff.
    last_attempt: Option<Instant>,
    fetch_in_flight: bool,
}

enum KeyPlan {
    UseCached,
    Fetch,
    WaitForFetch,
    RetryLater,
}

impl KeyCache {
    fn plan(&self, kid: &str, now: Instant) -> KeyPlan {
        let attempted_within = |interval| {
            self.last_attempt
                .is_some_and(|at| now.duration_since(at) < interval)
        };
        match &self.keys {
            None if self.fetch_in_flight => KeyPlan::WaitForFetch,
            None if attempted_within(EMPTY_CACHE_RETRY_INTERVAL) => KeyPlan::RetryLater,
            None => KeyPlan::Fetch,
            // Stale keys are still better than failing every sign-in while Google is slow.
            Some(_) if self.fetch_in_flight => KeyPlan::UseCached,
            Some(cached) => {
                let wants_fetch = cached.expires_at <= now || cached.keys.find(kid).is_none();
                if wants_fetch && !attempted_within(MIN_REFETCH_INTERVAL) {
                    KeyPlan::Fetch
                } else {
                    KeyPlan::UseCached
                }
            }
        }
    }
}

/// Verifies Google ID tokens. `verify` may block on HTTP when keys are stale: call it from a
/// blocking context. At most one thread fetches at a time, and never while holding the lock.
pub(crate) struct GoogleVerifier {
    client_id: String,
    source: Box<dyn GoogleKeySource>,
    cache: Mutex<KeyCache>,
    fetch_finished: Condvar,
}

impl std::fmt::Debug for GoogleVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoogleVerifier")
            .field("client_id", &self.client_id)
            .finish_non_exhaustive()
    }
}

impl GoogleVerifier {
    pub(crate) fn new(client_id: String, source: Box<dyn GoogleKeySource>) -> Self {
        Self {
            client_id,
            source,
            cache: Mutex::new(KeyCache::default()),
            fetch_finished: Condvar::new(),
        }
    }

    pub(crate) fn client_id(&self) -> &str {
        &self.client_id
    }

    pub(crate) fn verify(&self, id_token: &str) -> Result<GoogleIdentity, AuthError> {
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

    fn lock(&self) -> MutexGuard<'_, KeyCache> {
        self.cache.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn decoding_key(&self, kid: &str) -> Result<DecodingKey, AuthError> {
        let wait_until = Instant::now() + FETCH_WAIT;
        let mut cache = self.lock();
        loop {
            let now = Instant::now();
            match cache.plan(kid, now) {
                KeyPlan::UseCached => break,
                KeyPlan::RetryLater => return Err(AuthError::GoogleKeysRetryLater),
                KeyPlan::WaitForFetch => {
                    let Some(left) = wait_until.checked_duration_since(now) else {
                        return Err(AuthError::GoogleKeysRetryLater);
                    };
                    cache = self
                        .fetch_finished
                        .wait_timeout(cache, left)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0;
                }
                KeyPlan::Fetch => {
                    cache.fetch_in_flight = true;
                    cache.last_attempt = Some(now);
                    drop(cache);
                    let fetched = self.source.fetch();
                    cache = self.lock();
                    cache.fetch_in_flight = false;
                    self.fetch_finished.notify_all();
                    match fetched {
                        Ok(fetched) => {
                            cache.keys = Some(CachedKeys {
                                keys: fetched.keys,
                                expires_at: Instant::now() + fetched.max_age,
                            });
                        }
                        Err(error) if cache.keys.is_none() => {
                            return Err(AuthError::GoogleKeysUnavailable(error));
                        }
                        Err(error) => {
                            tracing::warn!(%error, "refreshing Google keys failed, using cached keys");
                        }
                    }
                    break;
                }
            }
        }
        let jwk = cache
            .keys
            .as_ref()
            .and_then(|cached| cached.keys.find(kid))
            .ok_or(AuthError::UnknownGoogleKey)?;
        DecodingKey::from_jwk(jwk).map_err(AuthError::InvalidGoogleToken)
    }
}

/// Fetches Google's JWKS over HTTPS.
#[derive(Debug)]
pub(crate) struct HttpGoogleKeySource {
    url: String,
}

impl HttpGoogleKeySource {
    pub(crate) const GOOGLE_CERTS_URL: &'static str = "https://www.googleapis.com/oauth2/v3/certs";

    pub(crate) fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

impl GoogleKeySource for HttpGoogleKeySource {
    fn fetch(&self) -> Result<FetchedKeys, KeyFetchError> {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .build()
            .new_agent();
        let mut response = agent
            .get(&self.url)
            .call()
            .map_err(KeyFetchError::Request)?;
        let max_age = response
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok())
            .and_then(parse_max_age)
            .unwrap_or(Duration::from_secs(3600));
        let keys: JwkSet = response
            .body_mut()
            .read_json()
            .map_err(KeyFetchError::Body)?;
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
