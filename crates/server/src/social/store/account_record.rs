use mg_protocol::proto;

use crate::social::AccountId;

/// What the server remembers of an account: enough to show it in a friend list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AccountRecord {
    pub(crate) id: AccountId,
    pub(crate) display_name: String,
    pub(crate) avatar_url: String,
}

impl AccountRecord {
    pub(crate) fn to_identity(&self) -> proto::Identity {
        proto::Identity {
            kind: proto::IdentityKind::Google as i32,
            id: self.id.as_str().to_owned(),
            display_name: self.display_name.clone(),
            avatar_url: self.avatar_url.clone(),
        }
    }
}
