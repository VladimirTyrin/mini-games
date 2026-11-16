use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as ClientPayload;

use super::store::AccountRecord;
use super::{AccountId, InviteLobby, Requester};
use crate::ids::{InviteId, SessionId};

#[derive(Debug)]
pub(crate) enum SocialCommand {
    /// A friends, push or invitation request. `lobby` is the requester's lobby (invitations).
    Request {
        from: Requester,
        message: ClientPayload,
        lobby: Option<InviteLobby>,
    },
    /// A Google sign-in: remember the account's name and picture.
    SignedIn(AccountRecord),
    /// Sent by the hub whenever an account's presence may have changed (it may not have).
    Presence {
        account: AccountId,
        presence: proto::Presence,
    },
    SessionEnded(SessionId),
    InviteExpired(InviteId),
    Housekeeping,
}
