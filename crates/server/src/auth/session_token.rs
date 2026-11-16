use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use super::auth_error::AuthError;
use super::avatar_url::AvatarUrl;
use super::display_name::DisplayName;

type HmacSha256 = Hmac<Sha256>;

/// v2 added the avatar; older tokens are refused and their owners sign in again.
const TOKEN_VERSION: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SessionKind {
    Google,
    BotClient,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SessionClaims {
    pub kind: SessionKind,
    pub subject: String,
    pub display_name: DisplayName,
    pub avatar: Option<AvatarUrl>,
    pub expires_at_unix: u64,
}

#[derive(Serialize, Deserialize)]
struct Payload {
    v: u8,
    kind: SessionKind,
    sub: String,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    avatar: Option<String>,
    exp: u64,
}

/// Stateless session tokens: `base64url(json payload) . base64url(HMAC-SHA256)`. There is no
/// database, so revocation is only possible by rotating the secret.
pub(crate) struct SessionTokens {
    secret: Vec<u8>,
    lifetime_secs: u64,
}

impl std::fmt::Debug for SessionTokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionTokens")
            .field("lifetime_secs", &self.lifetime_secs)
            .finish_non_exhaustive()
    }
}

impl SessionTokens {
    pub(crate) fn new(secret: Vec<u8>, lifetime_secs: u64) -> Self {
        Self {
            secret,
            lifetime_secs,
        }
    }

    fn mac(&self) -> HmacSha256 {
        // HMAC accepts keys of any length, so this cannot fail.
        HmacSha256::new_from_slice(&self.secret)
            .unwrap_or_else(|_| unreachable!("HMAC takes any key length"))
    }

    pub(crate) fn issue(
        &self,
        kind: SessionKind,
        subject: &str,
        display_name: &DisplayName,
        avatar: Option<&AvatarUrl>,
        now_unix: u64,
    ) -> String {
        let payload = Payload {
            v: TOKEN_VERSION,
            kind,
            sub: subject.to_owned(),
            name: display_name.as_str().to_owned(),
            avatar: avatar.map(|avatar| avatar.as_str().to_owned()),
            exp: now_unix.saturating_add(self.lifetime_secs),
        };
        let json = serde_json::to_vec(&payload).unwrap_or_default();
        let encoded = URL_SAFE_NO_PAD.encode(json);
        let mut mac = self.mac();
        mac.update(encoded.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        format!("{encoded}.{signature}")
    }

    pub(crate) fn verify(&self, token: &str, now_unix: u64) -> Result<SessionClaims, AuthError> {
        let (encoded, signature) = token
            .split_once('.')
            .ok_or(AuthError::InvalidSessionToken)?;
        let signature = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| AuthError::InvalidSessionToken)?;
        let mut mac = self.mac();
        mac.update(encoded.as_bytes());
        // Constant-time comparison.
        mac.verify_slice(&signature)
            .map_err(|_| AuthError::InvalidSessionToken)?;
        let json = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| AuthError::InvalidSessionToken)?;
        let payload: Payload =
            serde_json::from_slice(&json).map_err(|_| AuthError::InvalidSessionToken)?;
        if payload.v != TOKEN_VERSION {
            return Err(AuthError::InvalidSessionToken);
        }
        if payload.exp <= now_unix {
            return Err(AuthError::SessionExpired);
        }
        Ok(SessionClaims {
            kind: payload.kind,
            subject: payload.sub,
            display_name: DisplayName::sanitize(&payload.name, "Player"),
            avatar: payload.avatar.as_deref().and_then(AvatarUrl::parse),
            expires_at_unix: payload.exp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens() -> SessionTokens {
        SessionTokens::new(b"0123456789abcdef0123456789abcdef".to_vec(), 3600)
    }

    fn signed(tokens: &SessionTokens, payload: &[u8]) -> String {
        let encoded = URL_SAFE_NO_PAD.encode(payload);
        let mut mac = tokens.mac();
        mac.update(encoded.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        format!("{encoded}.{signature}")
    }

    #[test]
    fn issued_token_verifies_until_expiry() {
        let name = DisplayName::parse("Alice").unwrap();
        let avatar = AvatarUrl::parse("https://lh3.googleusercontent.com/a/alice").unwrap();
        let token = tokens().issue(SessionKind::Google, "sub-1", &name, Some(&avatar), 1_000);
        let claims = tokens().verify(&token, 2_000).unwrap();
        assert_eq!(claims.subject, "sub-1");
        assert_eq!(claims.display_name, name);
        assert_eq!(claims.avatar, Some(avatar));
        assert_eq!(claims.kind, SessionKind::Google);
        assert!(matches!(
            tokens().verify(&token, 4_600),
            Err(AuthError::SessionExpired)
        ));
    }

    #[test]
    fn token_without_avatar_round_trips_without_one() {
        let name = DisplayName::parse("bot-1").unwrap();
        let token = tokens().issue(SessionKind::BotClient, "bot-1", &name, None, 1_000);
        assert_eq!(tokens().verify(&token, 2_000).unwrap().avatar, None);
    }

    #[test]
    fn correctly_signed_version_1_token_is_rejected() {
        let tokens = tokens();
        let version_1 = signed(
            &tokens,
            br#"{"v":1,"kind":"google","sub":"sub-1","name":"Alice","exp":9999999999}"#,
        );
        assert!(matches!(
            tokens.verify(&version_1, 2_000),
            Err(AuthError::InvalidSessionToken)
        ));
        let version_2 = signed(
            &tokens,
            br#"{"v":2,"kind":"google","sub":"sub-1","name":"Alice","exp":9999999999}"#,
        );
        assert!(tokens.verify(&version_2, 2_000).is_ok());
    }

    #[test]
    fn tampered_or_foreign_tokens_are_rejected() {
        let name = DisplayName::parse("Alice").unwrap();
        let token = tokens().issue(SessionKind::Google, "sub-1", &name, None, 1_000);
        let (payload, signature) = token.split_once('.').unwrap();
        let forged_payload = URL_SAFE_NO_PAD
            .encode(br#"{"v":2,"kind":"google","sub":"admin","name":"x","exp":9999999999}"#);
        assert!(
            tokens()
                .verify(&format!("{forged_payload}.{signature}"), 2_000)
                .is_err()
        );
        let other_server = SessionTokens::new(b"another-secret-another-secret-xx".to_vec(), 3600);
        assert!(other_server.verify(&token, 2_000).is_err());
        assert!(tokens().verify(payload, 2_000).is_err());
        assert!(tokens().verify("garbage", 2_000).is_err());
    }
}
