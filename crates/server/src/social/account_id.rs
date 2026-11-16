use std::fmt;
use std::sync::Arc;

use crate::player_id::PlayerId;

/// A Google account, as other players see it: `g_` and 32 hex digits (see `PlayerIdKey`). A
/// device of the account plays as `<account>.<session>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct AccountId(Arc<str>);

#[derive(Debug, thiserror::Error)]
#[error("not a Google account id")]
pub(crate) struct AccountIdError;

impl AccountId {
    const PREFIX: &str = "g_";
    const DIGEST_CHARS: usize = 32;

    /// An account id, or the player id of one of its devices.
    pub(crate) fn parse(raw: &str) -> Result<Self, AccountIdError> {
        let account = raw
            .split_once('.')
            .map_or(raw, |(account, _device)| account);
        let valid = account.strip_prefix(Self::PREFIX).is_some_and(|digest| {
            digest.len() == Self::DIGEST_CHARS
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        });
        if valid {
            Ok(Self(account.into()))
        } else {
            Err(AccountIdError)
        }
    }

    /// The account a Google sign-in proved.
    pub(crate) fn of(account: &PlayerId) -> Result<Self, AccountIdError> {
        Self::parse(account.as_str())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCOUNT: &str = "g_0123456789abcdef0123456789abcdef";

    #[test]
    fn a_device_id_names_its_account() {
        let device = format!("{ACCOUNT}.0011223344556677");

        assert_eq!(AccountId::parse(&device).unwrap().as_str(), ACCOUNT);
        assert_eq!(AccountId::parse(ACCOUNT).unwrap().as_str(), ACCOUNT);
    }

    #[test]
    fn guests_bots_and_garbage_are_not_accounts() {
        assert!(AccountId::parse("u_0011223344556677").is_err());
        assert!(AccountId::parse("b_0123456789abcdef0123456789abcdef").is_err());
        assert!(AccountId::parse("g_0123").is_err());
        assert!(AccountId::parse("g_0123456789ABCDEF0123456789ABCDEF").is_err());
        assert!(AccountId::parse("").is_err());
    }
}
