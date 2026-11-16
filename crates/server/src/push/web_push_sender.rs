use std::time::Duration;

use super::push_http::{push_agent, rejection};
use super::web_push_encryption::encrypt;
use super::{PushError, PushNotification, VapidKey, WebPushSubscription};
use crate::clock::unix_seconds;

const SERVICE: &str = "the Web Push service";

/// Sends Web Push messages (RFC 8030) signed with the server's VAPID key.
#[derive(Debug)]
pub(crate) struct WebPushSender {
    key: VapidKey,
    /// `https://<site>` or `mailto:`: who push services contact about this sender.
    subject: String,
    agent: ureq::Agent,
}

impl WebPushSender {
    /// Apple refuses VAPID tokens valid for more than a day.
    const TOKEN_LIFETIME: Duration = Duration::from_secs(12 * 3600);

    pub(crate) fn new(key: VapidKey, subject: String) -> Self {
        Self {
            key,
            subject,
            agent: push_agent(),
        }
    }

    pub(crate) fn public_key(&self) -> &str {
        self.key.public_key()
    }

    pub(crate) fn send(
        &self,
        subscription: &WebPushSubscription,
        notification: &PushNotification,
    ) -> Result<(), PushError> {
        let body = encrypt(
            notification.web_push_payload().as_bytes(),
            &subscription.client_key,
            &subscription.auth_secret,
        )
        .map_err(PushError::Encryption)?;
        let expires_at = unix_seconds() + Self::TOKEN_LIFETIME.as_secs();
        let authorization =
            self.key
                .authorization(subscription.endpoint.origin(), &self.subject, expires_at);
        let response = self
            .agent
            .post(subscription.endpoint.url())
            .header("authorization", authorization)
            .header("ttl", notification.time_to_live.as_secs().to_string())
            .header("urgency", "high")
            .header("content-encoding", "aes128gcm")
            .header("content-type", "application/octet-stream")
            .send(&body[..])
            .map_err(|source| PushError::Request {
                service: SERVICE,
                source,
            })?;
        match response.status().as_u16() {
            200..=202 => Ok(()),
            404 | 410 => Err(PushError::TargetGone),
            _ => Err(rejection(SERVICE, response)),
        }
    }
}
