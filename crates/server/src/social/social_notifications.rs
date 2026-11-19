//! The push notifications the social actor sends. `kind` in the data tells the app what a tap
//! opens; `sentAt` is the server clock (unix ms).

use std::collections::BTreeMap;
use std::time::Duration;

use mg_protocol::proto;

use super::Invite;
use crate::push::{PushNotification, PushText};

/// A friend request or a new friend is news for a while, unlike an invitation.
const FRIEND_NEWS_LIFETIME: Duration = Duration::from_secs(24 * 3600);

pub(super) fn invitation(
    invite: &Invite,
    sent_at_ms: i64,
    time_to_live: Duration,
) -> PushNotification {
    PushNotification {
        title: PushText::InviteTitle {
            from: invite.from.identity.display_name.clone(),
        },
        body: PushText::InviteBody {
            game: invite.lobby.game,
        },
        tag: invite_tag(invite),
        data: BTreeMap::from([
            ("kind", "invite".to_owned()),
            ("inviteId", invite.id.as_str().to_owned()),
            ("lobbyId", invite.lobby.id.as_str().to_owned()),
            ("sentAt", sent_at_ms.to_string()),
        ]),
        time_to_live,
    }
}

/// Replaces the invitation on the devices that did not answer it.
pub(super) fn invitation_answered_elsewhere(
    invite: &Invite,
    state: proto::InviteState,
    sent_at_ms: i64,
) -> PushNotification {
    let from = invite.from.identity.display_name.clone();
    let body = if state == proto::InviteState::Accepted {
        PushText::InviteAcceptedElsewhere { from }
    } else {
        PushText::InviteDeclinedElsewhere { from }
    };
    PushNotification {
        title: PushText::AppTitle,
        body,
        tag: invite_tag(invite),
        data: BTreeMap::from([
            ("kind", "invite-closed".to_owned()),
            ("inviteId", invite.id.as_str().to_owned()),
            ("sentAt", sent_at_ms.to_string()),
        ]),
        time_to_live: Duration::from_secs(60),
    }
}

pub(super) fn friend_request(from: &proto::Identity, sent_at_ms: i64) -> PushNotification {
    PushNotification {
        title: PushText::AppTitle,
        body: PushText::FriendRequest {
            from: from.display_name.clone(),
        },
        tag: "friends".to_owned(),
        data: friends_data(sent_at_ms),
        time_to_live: FRIEND_NEWS_LIFETIME,
    }
}

pub(super) fn new_friend(friend: &proto::Identity, sent_at_ms: i64) -> PushNotification {
    PushNotification {
        title: PushText::AppTitle,
        body: PushText::NewFriend {
            friend: friend.display_name.clone(),
        },
        tag: "friends".to_owned(),
        data: friends_data(sent_at_ms),
        time_to_live: FRIEND_NEWS_LIFETIME,
    }
}

fn friends_data(sent_at_ms: i64) -> BTreeMap<&'static str, String> {
    BTreeMap::from([
        ("kind", "friends".to_owned()),
        ("sentAt", sent_at_ms.to_string()),
    ])
}

fn invite_tag(invite: &Invite) -> String {
    format!("invite-{}", invite.id)
}
