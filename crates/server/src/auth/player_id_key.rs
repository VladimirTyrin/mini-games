use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use super::display_name::DisplayName;
use crate::player_id::PlayerId;

type HmacSha256 = Hmac<Sha256>;

/// Bytes of the HMAC kept in an account's id: 128 bits, collision-free in practice.
const ID_DIGEST_BYTES: usize = 16;

/// Derives account player ids with an HMAC keyed by the server's session secret: the same
/// account always gets the same id, and other players cannot recover the Google subject or the
/// bot's configured name from it.
pub(crate) struct PlayerIdKey {
    secret: Vec<u8>,
}

impl std::fmt::Debug for PlayerIdKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerIdKey").finish_non_exhaustive()
    }
}

impl PlayerIdKey {
    pub(crate) fn new(secret: Vec<u8>) -> Self {
        Self { secret }
    }

    pub(crate) fn google(&self, subject: &str) -> PlayerId {
        self.derive("g_", &format!("google:{subject}"))
    }

    /// Case-insensitive, like the bot accounts themselves.
    pub(crate) fn bot_client(&self, name: &DisplayName) -> PlayerId {
        self.derive("b_", &format!("bot:{}", name.as_str().to_lowercase()))
    }

    fn derive(&self, prefix: &'static str, account: &str) -> PlayerId {
        // HMAC accepts keys of any length, so this cannot fail.
        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .unwrap_or_else(|_| unreachable!("HMAC takes any key length"));
        mac.update(account.as_bytes());
        let digest = mac.finalize().into_bytes();
        PlayerId::from_account_digest(prefix, digest.get(..ID_DIGEST_BYTES).unwrap_or(&digest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(secret: &[u8]) -> PlayerIdKey {
        PlayerIdKey::new(secret.to_vec())
    }

    #[test]
    fn google_ids_are_stable_opaque_and_keyed() {
        let google = key(b"secret-1").google("113355779");
        assert_eq!(google, key(b"secret-1").google("113355779"));
        assert!(google.as_str().starts_with("g_"));
        assert_eq!(google.as_str().len(), 2 + 2 * ID_DIGEST_BYTES);
        assert!(!google.as_str().contains("113355779"));
        assert_ne!(google, key(b"secret-2").google("113355779"));
        assert_ne!(google, key(b"secret-1").google("113355780"));
    }

    #[test]
    fn bot_ids_ignore_name_case() {
        let id = key(b"s").bot_client(&DisplayName::parse("Alpha").unwrap());
        assert!(id.as_str().starts_with("b_"));
        assert_eq!(
            id,
            key(b"s").bot_client(&DisplayName::parse("alpha").unwrap())
        );
    }
}
