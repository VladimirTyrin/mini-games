use std::fmt;
use std::sync::Arc;

use crate::ids::SessionId;

/// `Identity.id`: what tells players apart. Display names are only labels and may repeat.
/// Opaque, with a prefix per kind; an account gets the same id on every session and device.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PlayerId(Arc<str>);

impl PlayerId {
    pub(crate) fn guest(session: &SessionId) -> Self {
        Self(format!("u_{session}").into())
    }

    /// `digest` must be a keyed hash of the account (see `PlayerIdKey`), so the id is stable
    /// without revealing the account it was derived from.
    pub(crate) fn from_account_digest(prefix: &'static str, digest: &[u8]) -> Self {
        let mut id = String::with_capacity(prefix.len() + digest.len() * 2);
        id.push_str(prefix);
        for byte in digest {
            id.push_str(&format!("{byte:02x}"));
        }
        Self(id.into())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
