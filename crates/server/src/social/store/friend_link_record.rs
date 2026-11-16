use mg_protocol::proto;

use super::FriendLinkKind;
use crate::ids::FriendLinkCode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FriendLinkRecord {
    pub(crate) code: FriendLinkCode,
    pub(crate) kind: FriendLinkKind,
    pub(crate) expires_ms: i64,
}

impl FriendLinkRecord {
    pub(crate) fn to_wire(&self) -> proto::FriendLink {
        proto::FriendLink {
            code: self.code.as_str().to_owned(),
            kind: self.kind.to_wire() as i32,
            expires_at_unix_ms: u64::try_from(self.expires_ms).unwrap_or(0),
        }
    }
}
