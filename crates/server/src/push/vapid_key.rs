use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};

use crate::hex::decode_hex;

/// The server's VAPID key (RFC 8292): push services accept a message only with a JWT signed
/// by the key the browser subscribed with.
pub(crate) struct VapidKey {
    signing: SigningKey,
    /// Uncompressed public key, base64url: what `PushManager.subscribe` takes as
    /// `applicationServerKey` and what goes into every `Authorization` header.
    public_key: String,
}

#[derive(Debug, thiserror::Error)]
#[error("MG_VAPID_PRIVATE_KEY must be 64 hex characters of a P-256 private key")]
pub struct VapidKeyError;

impl std::fmt::Debug for VapidKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VapidKey")
            .field("public_key", &self.public_key)
            .finish_non_exhaustive()
    }
}

impl VapidKey {
    pub(crate) fn from_hex(hex: &str) -> Result<Self, VapidKeyError> {
        let bytes = decode_hex(hex.trim()).ok_or(VapidKeyError)?;
        if bytes.len() != 32 {
            return Err(VapidKeyError);
        }
        let signing = SigningKey::from_slice(&bytes).map_err(|_| VapidKeyError)?;
        let public_key =
            URL_SAFE_NO_PAD.encode(signing.verifying_key().to_sec1_point(false).as_bytes());
        Ok(Self {
            signing,
            public_key,
        })
    }

    pub(crate) fn public_key(&self) -> &str {
        &self.public_key
    }

    /// The `Authorization` header for a push service at `audience` (its origin).
    pub(crate) fn authorization(&self, audience: &str, subject: &str, expires_at: u64) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"typ":"JWT","alg":"ES256"}"#);
        let claims = serde_json::json!({ "aud": audience, "exp": expires_at, "sub": subject });
        let claims = URL_SAFE_NO_PAD.encode(claims.to_string());
        let signing_input = format!("{header}.{claims}");
        let signature: Signature = self.signing.sign(signing_input.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(signature.to_bytes());
        format!("vapid t={signing_input}.{signature}, k={}", self.public_key)
    }
}

#[cfg(test)]
mod tests {
    use p256::ecdsa::VerifyingKey;
    use p256::ecdsa::signature::Verifier;

    use super::*;

    const KEY_HEX: &str = "c9f58f89813e9f8e872e71f42aa64e1757c9254dcc62b72ddc010bb4043ea11c";

    #[test]
    fn signs_a_token_that_its_public_key_verifies() {
        let key = VapidKey::from_hex(KEY_HEX).unwrap();

        let header = key.authorization("https://web.push.apple.com", "https://example.org", 99);

        let (token, public_key) = header
            .strip_prefix("vapid t=")
            .and_then(|rest| rest.split_once(", k="))
            .unwrap();
        assert_eq!(public_key, key.public_key());
        let (signing_input, signature) = token.rsplit_once('.').unwrap();
        let verifying =
            VerifyingKey::from_sec1_bytes(&URL_SAFE_NO_PAD.decode(public_key).unwrap()).unwrap();
        let signature = Signature::from_slice(&URL_SAFE_NO_PAD.decode(signature).unwrap()).unwrap();
        verifying
            .verify(signing_input.as_bytes(), &signature)
            .unwrap();
        let claims = signing_input.split_once('.').unwrap().1;
        let claims: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(claims).unwrap()).unwrap();
        assert_eq!(
            claims,
            serde_json::json!({ "aud": "https://web.push.apple.com", "exp": 99, "sub": "https://example.org" })
        );
    }

    #[test]
    fn rejects_a_key_that_is_not_32_hex_bytes() {
        assert!(VapidKey::from_hex("abcd").is_err());
        assert!(VapidKey::from_hex(&"zz".repeat(32)).is_err());
        assert!(VapidKey::from_hex(&"00".repeat(32)).is_err());
    }
}
