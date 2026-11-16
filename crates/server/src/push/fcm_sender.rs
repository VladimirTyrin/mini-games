use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::Deserialize;

use super::push_http::{push_agent, rejection};
use super::{FcmToken, PushError, PushNotification};
use crate::clock::unix_seconds;

const FCM: &str = "FCM";
const FCM_SIGN_IN: &str = "Google sign-in for FCM";

/// Sends FCM messages (HTTP v1 API) as the Firebase project's service account.
pub(crate) struct FcmSender {
    send_url: String,
    client_email: String,
    token_uri: String,
    signing_key: EncodingKey,
    agent: ureq::Agent,
    access_token: Mutex<Option<AccessToken>>,
}

struct AccessToken {
    value: String,
    renew_after: Instant,
}

/// The fields used from the service account key file Firebase hands out.
#[derive(Deserialize)]
struct ServiceAccount {
    project_id: String,
    client_email: String,
    private_key: String,
    token_uri: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum FcmConfigError {
    #[error("MG_FCM_SERVICE_ACCOUNT is not base64")]
    Base64(#[source] base64::DecodeError),
    #[error("MG_FCM_SERVICE_ACCOUNT is not a service account key file")]
    Json(#[source] serde_json::Error),
    #[error("the service account's private key is unusable")]
    PrivateKey(#[source] jsonwebtoken::errors::Error),
    #[error("the service account's token_uri must be https")]
    TokenUri,
}

impl std::fmt::Debug for FcmSender {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FcmSender")
            .field("send_url", &self.send_url)
            .field("client_email", &self.client_email)
            .finish_non_exhaustive()
    }
}

impl FcmSender {
    const SCOPE: &str = "https://www.googleapis.com/auth/firebase.messaging";
    const TOKEN_LIFETIME: Duration = Duration::from_secs(3600);
    /// Renew this long before Google's expiry, so a token never expires mid-request.
    const RENEW_EARLY: Duration = Duration::from_secs(300);

    /// `service_account_base64`: the key file as base64 (an environment variable holds one line).
    pub(crate) fn from_base64(service_account_base64: &str) -> Result<Self, FcmConfigError> {
        let json = STANDARD
            .decode(service_account_base64.trim())
            .map_err(FcmConfigError::Base64)?;
        let account: ServiceAccount =
            serde_json::from_slice(&json).map_err(FcmConfigError::Json)?;
        if !account.token_uri.starts_with("https://") {
            return Err(FcmConfigError::TokenUri);
        }
        let signing_key = EncodingKey::from_rsa_pem(account.private_key.as_bytes())
            .map_err(FcmConfigError::PrivateKey)?;
        Ok(Self {
            send_url: format!(
                "https://fcm.googleapis.com/v1/projects/{}/messages:send",
                account.project_id
            ),
            client_email: account.client_email,
            token_uri: account.token_uri,
            signing_key,
            agent: push_agent(),
            access_token: Mutex::new(None),
        })
    }

    pub(crate) fn send(
        &self,
        token: &FcmToken,
        notification: &PushNotification,
    ) -> Result<(), PushError> {
        let access_token = self.access_token()?;
        let message = serde_json::json!({
            "message": {
                "token": token.as_str(),
                "notification": { "title": notification.title, "body": notification.body },
                "data": notification.data,
                "android": {
                    "priority": "HIGH",
                    "ttl": format!("{}s", notification.time_to_live.as_secs()),
                    "notification": {
                        // Created by the app (push-notifications.ts) with high importance, so
                        // the notification pops up instead of only landing in the shade.
                        "channel_id": "invites",
                        "tag": notification.tag,
                    },
                },
            },
        });
        let response = self
            .agent
            .post(&self.send_url)
            .header("authorization", format!("Bearer {access_token}"))
            .send_json(&message)
            .map_err(|source| PushError::Request {
                service: FCM,
                source,
            })?;
        match response.status().as_u16() {
            200 => Ok(()),
            404 => Err(PushError::TargetGone),
            401 => {
                self.forget_access_token();
                Err(rejection(FCM, response))
            }
            _ => Err(rejection(FCM, response)),
        }
    }

    /// Holds the lock while signing in, so concurrent sends wait for one sign-in instead of
    /// each starting their own.
    fn access_token(&self) -> Result<String, PushError> {
        let mut cached = self
            .access_token
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(token) = cached
            .as_ref()
            .filter(|token| Instant::now() < token.renew_after)
        {
            return Ok(token.value.clone());
        }
        let token = self.sign_in()?;
        let value = token.value.clone();
        *cached = Some(token);
        Ok(value)
    }

    fn forget_access_token(&self) {
        *self
            .access_token
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = None;
    }

    /// OAuth 2.0 JWT bearer grant (RFC 7523) for the service account.
    fn sign_in(&self) -> Result<AccessToken, PushError> {
        let issued_at = unix_seconds();
        let claims = serde_json::json!({
            "iss": self.client_email,
            "scope": Self::SCOPE,
            "aud": self.token_uri,
            "iat": issued_at,
            "exp": issued_at + Self::TOKEN_LIFETIME.as_secs(),
        });
        let assertion =
            jsonwebtoken::encode(&Header::new(Algorithm::RS256), &claims, &self.signing_key)
                .map_err(PushError::Signing)?;
        let started = Instant::now();
        let mut response = self
            .agent
            .post(&self.token_uri)
            .send_form([
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", assertion.as_str()),
            ])
            .map_err(|source| PushError::Request {
                service: FCM_SIGN_IN,
                source,
            })?;
        if response.status().as_u16() != 200 {
            return Err(rejection(FCM_SIGN_IN, response));
        }
        let token: TokenResponse =
            response
                .body_mut()
                .read_json()
                .map_err(|source| PushError::Request {
                    service: FCM_SIGN_IN,
                    source,
                })?;
        let lifetime = Duration::from_secs(token.expires_in).saturating_sub(Self::RENEW_EARLY);
        Ok(AccessToken {
            value: token.access_token,
            renew_after: started + lifetime,
        })
    }
}
