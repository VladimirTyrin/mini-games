use super::{AccountRecord, FriendLinkRecord};

/// One account's side of the friend graph.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FriendGraph {
    pub(crate) friends: Vec<AccountRecord>,
    pub(crate) incoming_requests: Vec<AccountRecord>,
    pub(crate) outgoing_requests: Vec<AccountRecord>,
    pub(crate) links: Vec<FriendLinkRecord>,
}
