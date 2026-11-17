use std::collections::BTreeMap;

use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as ClientPayload;
use mg_registry::GameKind;

use super::hub::Hub;
use super::lobby_phase::LobbyPhase;
use crate::ids::{LobbyId, SessionId};
use crate::social::{AccountId, AccountRecord, InviteLobby, Requester, SocialCommand};

/// The hub's side of friends and invitations: it knows who is connected and in which lobby,
/// the social actor knows the rest.
impl Hub {
    /// Hands a social request to the social actor, vouching for the Google account behind it.
    pub(super) fn forward_to_social(
        &self,
        session_id: &SessionId,
        request_id: u32,
        message: ClientPayload,
    ) {
        let Some(session) = self.sessions.get(session_id) else {
            return;
        };
        let (Some(account), Some(outbound)) = (
            session
                .who
                .google_account()
                .and_then(|account| AccountId::of(account).ok()),
            session.outbound().cloned(),
        ) else {
            return self.reject(
                session_id,
                request_id,
                proto::ErrorCode::AccountRequired,
                "sign in with Google to have friends",
            );
        };
        let lobby = matches!(message, ClientPayload::InviteFriend(_))
            .then(|| session.lobby.as_ref())
            .flatten()
            .and_then(|lobby_id| self.lobbies.get(lobby_id))
            .and_then(|lobby| {
                Some(InviteLobby {
                    id: lobby.id.clone(),
                    name: lobby.name.clone(),
                    game: GameKind::of_settings(&lobby.settings).ok()?,
                })
            });
        let identity = proto::Identity {
            id: account.as_str().to_owned(),
            ..session.identity.clone()
        };
        self.social.send(SocialCommand::Request {
            from: Requester {
                session: session_id.clone(),
                account,
                identity,
                outbound,
                request_id,
            },
            message,
            lobby,
        });
    }

    pub(super) fn deliver_to_account(&self, account: &AccountId, message: &proto::ServerMessage) {
        let mut connected = 0;
        for session in self.sessions.values() {
            if session_account(&session.who).as_ref() == Some(account)
                && session.connection.is_some()
            {
                session.send(message);
                connected += 1;
            }
        }
        if let Some(proto::server_message::Message::GameInvite(invite)) = &message.message {
            tracing::info!(
                invite = invite.invite_id,
                %account,
                connected,
                "invitation shown on connected devices"
            );
        }
    }

    /// Playing beats online beats offline across the account's devices.
    fn presence_of(&self, account: &AccountId) -> proto::Presence {
        let mut presence = proto::Presence::Offline;
        for session in self.sessions.values() {
            if session.connection.is_none()
                || session_account(&session.who).as_ref() != Some(account)
            {
                continue;
            }
            let playing = session
                .lobby
                .as_ref()
                .and_then(|lobby| self.lobbies.get(lobby))
                .is_some_and(|lobby| matches!(lobby.phase, LobbyPhase::InMatch(_)));
            if playing {
                return proto::Presence::Playing;
            }
            presence = proto::Presence::Online;
        }
        presence
    }

    pub(super) fn presence_of_accounts(
        &self,
        accounts: Vec<AccountId>,
    ) -> BTreeMap<AccountId, proto::Presence> {
        accounts
            .into_iter()
            .map(|account| {
                let presence = self.presence_of(&account);
                (account, presence)
            })
            .collect()
    }

    /// Tells the social actor where this session's account stands now; it shares changes with
    /// the account's friends.
    pub(super) fn announce_presence(&self, session: &SessionId) {
        if let Some(account) = self
            .sessions
            .get(session)
            .and_then(|session| session_account(&session.who))
        {
            self.announce_account_presence(account);
        }
    }

    pub(super) fn announce_account_presence(&self, account: AccountId) {
        let presence = self.presence_of(&account);
        self.social
            .send(SocialCommand::Presence { account, presence });
    }

    pub(super) fn announce_lobby_presence(&self, lobby_id: &LobbyId) {
        if let Some(lobby) = self.lobbies.get(lobby_id) {
            for member in lobby.members() {
                self.announce_presence(member);
            }
        }
    }

    /// A Google sign-in: the social actor remembers the account's name and picture.
    pub(super) fn announce_sign_in(&self, session: &SessionId) {
        let Some(session) = self.sessions.get(session) else {
            return;
        };
        let Some(account) = session_account(&session.who) else {
            return;
        };
        self.social.send(SocialCommand::SignedIn(AccountRecord {
            id: account.clone(),
            display_name: session.identity.display_name.clone(),
            avatar_url: session.identity.avatar_url.clone(),
        }));
        self.announce_account_presence(account);
    }
}

fn session_account(who: &crate::authenticated::Authenticated) -> Option<AccountId> {
    who.google_account()
        .and_then(|account| AccountId::of(account).ok())
}
