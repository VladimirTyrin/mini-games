use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::auth_error::AuthError;
use super::display_name::DisplayName;
use crate::config::ConfigError;

/// One configured bot: its name and the hash of its key.
#[derive(Debug)]
pub(crate) struct BotKeyEntry {
    name: DisplayName,
    key_sha256: [u8; 32],
    key_id: String,
}

impl BotKeyEntry {
    pub(crate) fn name(&self) -> &DisplayName {
        &self.name
    }

    /// Names this key in the session tokens issued for it, so a new key for the same name
    /// revokes them. A hash of the configured hash: the token never carries the hash itself.
    pub(crate) fn key_id(&self) -> &str {
        &self.key_id
    }
}

/// Network-bot keys from configuration. Only SHA-256 hashes are configured, so the config file
/// never contains usable secrets.
#[derive(Debug, Default)]
pub(crate) struct BotKeys {
    entries: Vec<BotKeyEntry>,
}

impl BotKeys {
    /// Format: `name:sha256hex,name2:sha256hex`.
    pub(crate) fn parse(config: &str) -> Result<Self, ConfigError> {
        let mut entries = Vec::new();
        for item in config
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
        {
            let invalid = |reason| ConfigError::BotKey {
                entry: item.to_owned(),
                reason,
            };
            let (name, hash_hex) = item
                .split_once(':')
                .ok_or_else(|| invalid("expected name:sha256hex"))?;
            let name = DisplayName::parse(name).map_err(|_| invalid("invalid bot name"))?;
            let key_sha256 = parse_sha256_hex(hash_hex)
                .ok_or_else(|| invalid("hash must be 64 hex characters"))?;
            let key_id = Sha256::digest(key_sha256)
                .iter()
                .take(16)
                .map(|byte| format!("{byte:02x}"))
                .collect();
            entries.push(BotKeyEntry {
                name,
                key_sha256,
                key_id,
            });
        }
        Ok(Self { entries })
    }

    pub(crate) fn authenticate(&self, key: &str) -> Result<&BotKeyEntry, AuthError> {
        let digest: [u8; 32] = Sha256::digest(key.as_bytes()).into();
        self.entries
            .iter()
            .find(|entry| bool::from(entry.key_sha256.ct_eq(&digest)))
            .ok_or(AuthError::UnknownBotKey)
    }

    /// Session tokens outlive configuration changes: a token is good only while the key it was
    /// issued for is still configured for that name (removed or replaced keys revoke it).
    pub(crate) fn is_configured(&self, name: &DisplayName, key_id: &str) -> bool {
        self.entries
            .iter()
            .any(|entry| &entry.name == name && entry.key_id == key_id)
    }
}

fn parse_sha256_hex(hex: &str) -> Option<[u8; 32]> {
    let hex = hex.trim();
    if hex.len() != 64 || !hex.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    for (byte, pair) in out.iter_mut().zip(hex.as_bytes().as_chunks::<2>().0) {
        let text = std::str::from_utf8(pair).ok()?;
        *byte = u8::from_str_radix(text, 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authenticates_by_hash() {
        // sha256("secret")
        let keys = BotKeys::parse(
            "alpha:2bb80d537b1da3e38bd30361aa855686bde0eacd7162fef6a25fe97bf527a25b",
        )
        .unwrap();
        let alpha = keys.authenticate("secret").unwrap();
        assert_eq!(alpha.name().as_str(), "alpha");
        assert!(keys.authenticate("wrong").is_err());
        assert!(keys.is_configured(alpha.name(), alpha.key_id()));
        assert!(!keys.is_configured(&DisplayName::parse("beta").unwrap(), alpha.key_id()));
    }

    #[test]
    fn a_new_key_for_the_same_name_revokes_the_old_one() {
        // sha256("secret"), then sha256("other").
        let old = BotKeys::parse(
            "alpha:2bb80d537b1da3e38bd30361aa855686bde0eacd7162fef6a25fe97bf527a25b",
        )
        .unwrap();
        let rotated = BotKeys::parse(
            "alpha:d9298a10d1b0735837dc4bd85dac641b0f3cef27a47e5d53a54f2f3f5b2fcffa",
        )
        .unwrap();
        let issued = old.authenticate("secret").unwrap();
        assert!(!rotated.is_configured(issued.name(), issued.key_id()));
        let current = rotated.authenticate("other").unwrap();
        assert!(rotated.is_configured(current.name(), current.key_id()));
    }

    #[test]
    fn rejects_malformed_config() {
        assert!(BotKeys::parse("alpha").is_err());
        assert!(BotKeys::parse("alpha:1234").is_err());
        assert!(BotKeys::parse("").unwrap().authenticate("x").is_err());
    }
}
