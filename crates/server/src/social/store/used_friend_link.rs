use crate::social::AccountId;

/// Whose link was used, so both sides can be told.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UsedFriendLink {
    pub(crate) owner: AccountId,
}
