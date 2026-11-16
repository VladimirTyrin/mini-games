#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FriendRequestOutcome {
    Sent,
    /// The other account had asked first: now they are friends.
    BecameFriends,
    AlreadySent,
    AlreadyFriends,
}
