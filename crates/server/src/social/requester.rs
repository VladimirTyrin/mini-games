use mg_protocol::proto;

use super::AccountId;
use crate::ids::SessionId;
use crate::outbound::Outbound;

/// A signed-in Google session asking the social actor for something; the hub vouches for it.
#[derive(Clone, Debug)]
pub(crate) struct Requester {
    pub(crate) session: SessionId,
    pub(crate) account: AccountId,
    /// The account as friends see it (its id is the account id, not the device's).
    pub(crate) identity: proto::Identity,
    pub(crate) outbound: Outbound,
    pub(crate) request_id: u32,
}
