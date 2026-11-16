#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FriendLinkRefusal {
    /// Unknown, used up, revoked or expired: the link cannot tell which, and need not.
    Invalid,
    OwnLink,
}
