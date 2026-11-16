use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::auth_error::AuthError;
use super::display_name::DisplayName;

#[derive(Debug)]
struct BotKeyEntry {
    name: DisplayName,
    key_sha256: [u8; 32],
}

/// Network-bot keys from configuration. Only SHA-256 hashes are configured, so the config file
/// never contains usable secrets.
#[derive(Debug, Default)]
pub struct BotKeys {
    entries: Vec<BotKeyEntry>,
}

impl BotKeys {
    /// Format: `name:sha256hex,name2:sha256hex`.
    pub fn parse(config: &str) -> Result<Self, String> {
        let mut entries = Vec::new();
        for item in config
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
        {
            let (name, hash_hex) = item
                .split_once(':')
                .ok_or_else(|| format!("bot key `{item}`: expected name:sha256hex"))?;
            let name = DisplayName::parse(name).map_err(|e| format!("bot key `{item}`: {e}"))?;
            let key_sha256 = parse_sha256_hex(hash_hex)
                .ok_or_else(|| format!("bot key `{item}`: hash must be 64 hex chars"))?;
            entries.push(BotKeyEntry { name, key_sha256 });
        }
        Ok(Self { entries })
    }

    pub fn authenticate(&self, key: &str) -> Result<&DisplayName, AuthError> {
        let digest: [u8; 32] = Sha256::digest(key.as_bytes()).into();
        self.entries
            .iter()
            .find(|entry| bool::from(entry.key_sha256.ct_eq(&digest)))
            .map(|entry| &entry.name)
            .ok_or(AuthError::UnknownBotKey)
    }
}

fn parse_sha256_hex(hex: &str) -> Option<[u8; 32]> {
    let hex = hex.trim();
    if hex.len() != 64 || !hex.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    for (byte, pair) in out.iter_mut().zip(hex.as_bytes().chunks_exact(2)) {
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
        assert_eq!(keys.authenticate("secret").unwrap().as_str(), "alpha");
        assert!(keys.authenticate("wrong").is_err());
    }

    #[test]
    fn rejects_malformed_config() {
        assert!(BotKeys::parse("alpha").is_err());
        assert!(BotKeys::parse("alpha:1234").is_err());
        assert!(BotKeys::parse("").unwrap().authenticate("x").is_err());
    }
}
