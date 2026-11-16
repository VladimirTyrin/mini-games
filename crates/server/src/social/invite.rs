use mg_protocol::proto;

use super::{AccountId, InviteLobby, Requester};
use crate::ids::InviteId;

/// An open invitation into a lobby. Closed ones are dropped: answering one that is gone gets
/// `ERROR_CODE_INVITE_CLOSED`.
#[derive(Clone, Debug)]
pub(crate) struct Invite {
    pub(crate) id: InviteId,
    pub(crate) from: Requester,
    pub(crate) to: proto::Identity,
    pub(crate) to_account: AccountId,
    pub(crate) lobby: InviteLobby,
    pub(crate) expires_at_ms: i64,
}

impl Invite {
    pub(crate) fn update(&self, state: proto::InviteState, now_ms: i64) -> proto::InviteUpdate {
        proto::InviteUpdate {
            invite_id: self.id.as_str().to_owned(),
            friend: Some(self.to.clone()),
            state: state as i32,
            expires_at_unix_ms: u64::try_from(self.expires_at_ms).unwrap_or(0),
            expires_in_ms: self.time_left_ms(now_ms),
        }
    }

    pub(crate) fn to_wire(&self, now_ms: i64) -> proto::GameInvite {
        proto::GameInvite {
            invite_id: self.id.as_str().to_owned(),
            from: Some(self.from.identity.clone()),
            lobby_id: self.lobby.id.as_str().to_owned(),
            lobby_name: self.lobby.name.clone(),
            game: self.lobby.game.info().id.to_owned(),
            expires_at_unix_ms: u64::try_from(self.expires_at_ms).unwrap_or(0),
            expires_in_ms: self.time_left_ms(now_ms),
        }
    }

    fn time_left_ms(&self, now_ms: i64) -> u32 {
        u32::try_from(self.expires_at_ms.saturating_sub(now_ms).max(0)).unwrap_or(u32::MAX)
    }
}
