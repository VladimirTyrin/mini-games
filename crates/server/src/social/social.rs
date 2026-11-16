use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as ClientPayload;
use mg_protocol::proto::server_message::Message;
use proto::ErrorCode;
use tokio::sync::{mpsc, oneshot};

use super::notification_throttle::{NotificationKind, NotificationThrottle};
use super::social_notifications as notifications;
use super::store::{
    FriendLinkKind, FriendLinkRecord, FriendLinkRefusal, FriendRequestDecision,
    FriendRequestOutcome, Store, StoreError,
};
use super::{AccountId, Invite, InviteLobby, Requester, SocialCommand, SocialHandle};
use crate::clock::unix_millis;
use crate::codec;
use crate::error_chain::error_chain;
use crate::hub::{HubCommand, HubHandle};
use crate::ids::{FriendLinkCode, InviteId, SessionId};
use crate::push::{PushError, PushNotification, PushQueue, PushSender, PushTarget};
use crate::timing::Timing;

/// Friends, friend links, push targets and invitations. Runs as one task, like the hub, and
/// owns the database; it asks the hub who is online and has the hub deliver to an account's
/// devices. The hub never waits on it, so the two cannot deadlock.
pub(crate) struct Social {
    store: Arc<Store>,
    push: Arc<PushSender>,
    push_queue: PushQueue,
    hub: HubHandle,
    handle: SocialHandle,
    timing: Timing,
    /// Open invitations only; at most `MAX_OPEN_INVITES_PER_SESSION` per session.
    invites: HashMap<InviteId, Invite>,
    /// Where each device said it receives pushes: an answer on one device then notifies only
    /// the others.
    session_targets: HashMap<SessionId, String>,
    /// What friends were last told, so the hub's repeated reports are not broadcast again. Only
    /// accounts that are online: going offline removes the entry.
    presences: HashMap<AccountId, proto::Presence>,
    throttle: NotificationThrottle,
}

/// A refused request, sent back as `ErrorResponse`.
struct Refusal {
    code: ErrorCode,
    message: String,
}

impl Refusal {
    fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn internal(error: &StoreError) -> Self {
        tracing::error!(error = %error_chain(error), "database request failed");
        Self::new(ErrorCode::Internal, "the server could not do that")
    }
}

type Outcome = Result<Message, Refusal>;

fn ack() -> Outcome {
    Ok(Message::Ack(proto::Ack {}))
}

impl Social {
    /// Accounts with more open friend requests than this ask too much.
    const MAX_OUTGOING_REQUESTS: usize = 50;
    const MAX_ACTIVE_LINKS: usize = 20;
    const MAX_OPEN_INVITES_PER_SESSION: usize = 5;
    /// Keeps a friend list (and every presence change fanned out to it) a sensible size.
    const MAX_FRIENDS: usize = 500;
    const LINK_LIFETIME: Duration = Duration::from_secs(7 * 24 * 3600);

    pub(crate) fn new(
        store: Arc<Store>,
        push: Arc<PushSender>,
        push_queue: PushQueue,
        hub: HubHandle,
        handle: SocialHandle,
        timing: Timing,
    ) -> Self {
        Self {
            store,
            push,
            push_queue,
            hub,
            handle,
            timing,
            invites: HashMap::new(),
            session_targets: HashMap::new(),
            presences: HashMap::new(),
            throttle: NotificationThrottle::default(),
        }
    }

    pub(crate) async fn run(mut self, mut commands: mpsc::UnboundedReceiver<SocialCommand>) {
        while let Some(command) = commands.recv().await {
            match command {
                SocialCommand::Request {
                    from,
                    message,
                    lobby,
                } => {
                    let request_id = from.request_id;
                    let outbound = from.outbound.clone();
                    match self.request(from, message, lobby).await {
                        Ok(reply) => outbound.send(&codec::reply(request_id, reply)),
                        Err(refusal) => {
                            outbound.send(&codec::error(request_id, refusal.code, &refusal.message))
                        }
                    }
                }
                SocialCommand::SignedIn(account) => {
                    let now = unix_millis();
                    let id = account.id.clone();
                    if let Err(refusal) = self
                        .db(move |store| store.remember_account(&account, now))
                        .await
                    {
                        tracing::warn!(message = refusal.message, "account not remembered");
                    }
                    // A device that was away while an invitation came in still gets to answer it.
                    for invite in self
                        .invites
                        .values()
                        .filter(|invite| invite.to_account == id)
                    {
                        self.deliver(&id, Message::GameInvite(invite.to_wire(now)));
                    }
                }
                SocialCommand::Presence { account, presence } => {
                    self.presence_changed(account, presence).await;
                }
                SocialCommand::SessionEnded(session) => {
                    self.session_targets.remove(&session);
                    self.close_invites_from(&session);
                }
                SocialCommand::LeftLobby(session) => self.close_invites_from(&session),
                SocialCommand::SignedOut(session) => {
                    if let Some(address) = self.session_targets.remove(&session)
                        && let Err(refusal) = self
                            .db(move |store| store.forget_push_target(&address))
                            .await
                    {
                        tracing::warn!(message = refusal.message, "push target not forgotten");
                    }
                }
                SocialCommand::InviteExpired(id) => {
                    if let Some(invite) = self.invites.remove(&id) {
                        self.close_invite(&invite, proto::InviteState::Expired, None);
                    }
                }
                SocialCommand::Housekeeping => {
                    self.throttle.forget_expired(tokio::time::Instant::now());
                    let now = unix_millis();
                    if let Err(refusal) =
                        self.db(move |store| store.delete_expired_links(now)).await
                    {
                        tracing::warn!(message = refusal.message, "expired links not deleted");
                    }
                }
            }
        }
    }

    async fn request(
        &mut self,
        from: Requester,
        message: ClientPayload,
        lobby: Option<InviteLobby>,
    ) -> Outcome {
        match message {
            ClientPayload::RegisterPushTarget(register) => {
                self.register_push_target(&from, register).await
            }
            ClientPayload::ListFriends(_) => {
                Ok(Message::FriendList(self.friend_list(&from.account).await?))
            }
            ClientPayload::SendFriendRequest(request) => {
                self.send_friend_request(&from, &request.player_id).await
            }
            ClientPayload::AnswerFriendRequest(answer) => {
                self.answer_friend_request(&from, answer).await
            }
            ClientPayload::RemoveFriend(remove) => {
                self.remove_friend(&from, &remove.account_id).await
            }
            ClientPayload::CreateFriendLink(create) => {
                self.create_friend_link(&from, create.kind).await
            }
            ClientPayload::RevokeFriendLink(revoke) => {
                self.revoke_friend_link(&from, &revoke.code).await
            }
            ClientPayload::UseFriendLink(link) => self.use_friend_link(&from, &link.code).await,
            ClientPayload::InviteFriend(invite) => {
                self.invite_friend(from, &invite.account_id, lobby).await
            }
            ClientPayload::CancelInvite(cancel) => self.cancel_invite(&from, &cancel.invite_id),
            ClientPayload::AnswerInvite(answer) => self.answer_invite(&from, answer),
            // The hub routes only the messages above here.
            _ => Err(Refusal::new(
                ErrorCode::InvalidRequest,
                "not a social request",
            )),
        }
    }

    async fn db<T: Send + 'static>(
        &self,
        work: impl FnOnce(&Store) -> Result<T, StoreError> + Send + 'static,
    ) -> Result<T, Refusal> {
        let store = self.store.clone();
        match tokio::task::spawn_blocking(move || work(&store)).await {
            Ok(result) => result.map_err(|error| Refusal::internal(&error)),
            Err(error) => {
                tracing::error!(%error, "database task failed");
                Err(Refusal::new(
                    ErrorCode::Internal,
                    "the server could not do that",
                ))
            }
        }
    }

    async fn presence_of(&self, accounts: Vec<AccountId>) -> BTreeMap<AccountId, proto::Presence> {
        let (reply, answer) = oneshot::channel();
        self.hub.send(HubCommand::PresenceOf { accounts, reply });
        // Shutting down: everyone looks offline, which is about to be true.
        answer.await.unwrap_or_default()
    }

    fn deliver(&self, account: &AccountId, message: Message) {
        self.hub.send(HubCommand::DeliverToAccount {
            account: account.clone(),
            message: codec::push(message),
        });
    }

    fn deliver_to_session(&self, session: &SessionId, message: Message) {
        self.hub.send(HubCommand::DeliverToSession {
            session: session.clone(),
            message: codec::push(message),
        });
    }

    /// Refuses when either account has `MAX_FRIENDS` already.
    async fn has_room_for_a_friend(&self, accounts: [&AccountId; 2]) -> Result<(), Refusal> {
        for account in accounts {
            let owner = account.clone();
            if self.db(move |store| store.friend_ids(&owner)).await?.len() >= Self::MAX_FRIENDS {
                return Err(Refusal::new(ErrorCode::RateLimited, "too many friends"));
            }
        }
        Ok(())
    }

    /// Notifies unless this sender notified this account of the same thing just now.
    fn notify_once(
        &mut self,
        kind: NotificationKind,
        from: &AccountId,
        to: &AccountId,
        notification: PushNotification,
    ) {
        if self
            .throttle
            .allow(kind, from, to, tokio::time::Instant::now())
        {
            self.notify(to, notification, None);
        }
    }

    async fn friend_list(&self, account: &AccountId) -> Result<proto::FriendList, Refusal> {
        let now = unix_millis();
        let owner = account.clone();
        let graph = self
            .db(move |store| store.friend_graph(&owner, now))
            .await?;
        let presences = self
            .presence_of(
                graph
                    .friends
                    .iter()
                    .map(|friend| friend.id.clone())
                    .collect(),
            )
            .await;
        Ok(proto::FriendList {
            friends: graph
                .friends
                .iter()
                .map(|friend| proto::Friend {
                    identity: Some(friend.to_identity()),
                    presence: presences
                        .get(&friend.id)
                        .copied()
                        .unwrap_or(proto::Presence::Offline) as i32,
                })
                .collect(),
            incoming_requests: graph
                .incoming_requests
                .iter()
                .map(|account| account.to_identity())
                .collect(),
            outgoing_requests: graph
                .outgoing_requests
                .iter()
                .map(|account| account.to_identity())
                .collect(),
            links: graph.links.iter().map(FriendLinkRecord::to_wire).collect(),
        })
    }

    /// Pushes the current friend list to every connected device of these accounts.
    async fn refresh_friend_lists(&self, accounts: &[&AccountId]) {
        for account in accounts {
            match self.friend_list(account).await {
                Ok(list) => self.deliver(account, Message::FriendList(list)),
                Err(refusal) => tracing::warn!(message = refusal.message, "friend list not pushed"),
            }
        }
    }

    async fn register_push_target(
        &mut self,
        from: &Requester,
        register: proto::RegisterPushTarget,
    ) -> Outcome {
        let target = PushTarget::parse(register.target)
            .map_err(|error| Refusal::new(ErrorCode::InvalidRequest, error.to_string()))?;
        if !self.push.can_reach(&target) {
            return Err(Refusal::new(
                ErrorCode::PushUnavailable,
                "this server cannot send notifications to this device",
            ));
        }
        let address = target.address().to_owned();
        let account = from.account.clone();
        let now = unix_millis();
        self.db(move |store| store.save_push_target(&account, &target, now))
            .await?;
        self.session_targets.insert(from.session.clone(), address);
        ack()
    }

    /// Who `raw` names, if it is a known account other than the requester's.
    async fn other_account(&self, from: &Requester, raw: &str) -> Result<AccountId, Refusal> {
        let account = AccountId::parse(raw).map_err(|_| {
            Refusal::new(
                ErrorCode::AccountRequired,
                "only Google accounts can be friends",
            )
        })?;
        if account == from.account {
            return Err(Refusal::new(
                ErrorCode::InvalidRequest,
                "that is your own account",
            ));
        }
        let lookup = account.clone();
        if self
            .db(move |store| store.account(&lookup))
            .await?
            .is_none()
        {
            return Err(Refusal::new(ErrorCode::AccountNotFound, "unknown account"));
        }
        Ok(account)
    }

    async fn send_friend_request(&mut self, from: &Requester, player_id: &str) -> Outcome {
        let to = self.other_account(from, player_id).await?;
        self.has_room_for_a_friend([&from.account, &to]).await?;
        let sender = from.account.clone();
        let outgoing = self
            .db(move |store| store.friend_graph(&sender, 0))
            .await?
            .outgoing_requests
            .len();
        if outgoing >= Self::MAX_OUTGOING_REQUESTS {
            return Err(Refusal::new(
                ErrorCode::RateLimited,
                "too many unanswered friend requests",
            ));
        }
        let (sender, recipient, now) = (from.account.clone(), to.clone(), unix_millis());
        let outcome = self
            .db(move |store| store.send_friend_request(&sender, &recipient, now))
            .await?;
        match outcome {
            FriendRequestOutcome::Sent => self.notify_once(
                NotificationKind::FriendRequest,
                &from.account,
                &to,
                notifications::friend_request(&from.identity, now),
            ),
            FriendRequestOutcome::BecameFriends => self.notify_once(
                NotificationKind::NewFriend,
                &from.account,
                &to,
                notifications::new_friend(&from.identity, now),
            ),
            FriendRequestOutcome::AlreadySent | FriendRequestOutcome::AlreadyFriends => {}
        }
        self.refresh_friend_lists(&[&from.account, &to]).await;
        ack()
    }

    async fn answer_friend_request(
        &mut self,
        from: &Requester,
        answer: proto::AnswerFriendRequest,
    ) -> Outcome {
        let decision = match proto::FriendRequestAnswer::try_from(answer.answer) {
            Ok(proto::FriendRequestAnswer::Accept) => FriendRequestDecision::Accept,
            Ok(proto::FriendRequestAnswer::Decline) => FriendRequestDecision::Decline,
            _ => return Err(Refusal::new(ErrorCode::InvalidRequest, "accept or decline")),
        };
        let asker = self.other_account(from, &answer.account_id).await?;
        if decision == FriendRequestDecision::Accept {
            self.has_room_for_a_friend([&from.account, &asker]).await?;
        }
        let (recipient, sender, now) = (from.account.clone(), asker.clone(), unix_millis());
        let found = self
            .db(move |store| store.answer_friend_request(&recipient, &sender, decision, now))
            .await?;
        if !found {
            return Err(Refusal::new(
                ErrorCode::InvalidRequest,
                "no such friend request",
            ));
        }
        if decision == FriendRequestDecision::Accept {
            self.notify_once(
                NotificationKind::NewFriend,
                &from.account,
                &asker,
                notifications::new_friend(&from.identity, now),
            );
        }
        self.refresh_friend_lists(&[&from.account, &asker]).await;
        ack()
    }

    async fn remove_friend(&mut self, from: &Requester, raw: &str) -> Outcome {
        let other = self.other_account(from, raw).await?;
        let (mine, theirs) = (from.account.clone(), other.clone());
        self.db(move |store| store.remove_friend(&mine, &theirs))
            .await?;
        let between: Vec<InviteId> = self
            .invites
            .values()
            .filter(|invite| {
                (invite.from.account == from.account && invite.to_account == other)
                    || (invite.from.account == other && invite.to_account == from.account)
            })
            .map(|invite| invite.id.clone())
            .collect();
        for id in between {
            if let Some(invite) = self.invites.remove(&id) {
                self.close_invite(&invite, proto::InviteState::Cancelled, None);
            }
        }
        self.refresh_friend_lists(&[&from.account, &other]).await;
        ack()
    }

    async fn create_friend_link(&self, from: &Requester, kind: i32) -> Outcome {
        let kind = FriendLinkKind::from_wire(kind)
            .ok_or_else(|| Refusal::new(ErrorCode::InvalidRequest, "single-use or multi-use"))?;
        let now = unix_millis();
        let owner = from.account.clone();
        let active = self
            .db(move |store| store.active_link_count(&owner, now))
            .await?;
        if active >= Self::MAX_ACTIVE_LINKS {
            return Err(Refusal::new(
                ErrorCode::RateLimited,
                "too many open friend links: revoke some first",
            ));
        }
        let lifetime = i64::try_from(Self::LINK_LIFETIME.as_millis()).unwrap_or(i64::MAX);
        let link = FriendLinkRecord {
            code: FriendLinkCode::random(),
            kind,
            expires_ms: now.saturating_add(lifetime),
        };
        let (owner, stored) = (from.account.clone(), link.clone());
        self.db(move |store| store.create_friend_link(&owner, &stored))
            .await?;
        self.refresh_friend_lists(&[&from.account]).await;
        Ok(Message::FriendLink(link.to_wire()))
    }

    async fn revoke_friend_link(&self, from: &Requester, raw: &str) -> Outcome {
        let code = FriendLinkCode::parse(raw)
            .ok_or_else(|| Refusal::new(ErrorCode::InvalidRequest, "not a friend link"))?;
        let owner = from.account.clone();
        self.db(move |store| store.revoke_friend_link(&owner, &code))
            .await?;
        self.refresh_friend_lists(&[&from.account]).await;
        ack()
    }

    async fn use_friend_link(&mut self, from: &Requester, raw: &str) -> Outcome {
        let invalid = || {
            Refusal::new(
                ErrorCode::FriendLinkInvalid,
                "this friend link is used up or expired",
            )
        };
        let code = FriendLinkCode::parse(raw).ok_or_else(invalid)?;
        // Only this side can be checked before the link names its owner.
        self.has_room_for_a_friend([&from.account, &from.account])
            .await?;
        let (user, now) = (from.account.clone(), unix_millis());
        let used = self
            .db(move |store| store.use_friend_link(&code, &user, now))
            .await?;
        match used {
            Ok(link) => {
                if link.became_friends {
                    self.notify_once(
                        NotificationKind::NewFriend,
                        &from.account,
                        &link.owner,
                        notifications::new_friend(&from.identity, now),
                    );
                }
                self.refresh_friend_lists(&[&from.account, &link.owner])
                    .await;
                ack()
            }
            Err(FriendLinkRefusal::Invalid) => Err(invalid()),
            Err(FriendLinkRefusal::OwnLink) => Err(Refusal::new(
                ErrorCode::InvalidRequest,
                "this is your own friend link: send it to a friend",
            )),
        }
    }

    async fn invite_friend(
        &mut self,
        from: Requester,
        raw: &str,
        lobby: Option<InviteLobby>,
    ) -> Outcome {
        let lobby =
            lobby.ok_or_else(|| Refusal::new(ErrorCode::NotInLobby, "join a lobby first"))?;
        let friend = self.other_account(&from, raw).await?;
        let (mine, theirs) = (from.account.clone(), friend.clone());
        if !self
            .db(move |store| store.are_friends(&mine, &theirs))
            .await?
        {
            return Err(Refusal::new(
                ErrorCode::NotFriends,
                "you can only invite friends",
            ));
        }
        if let Some(open) = self.invites.values().find(|invite| {
            invite.from.session == from.session
                && invite.to_account == friend
                && invite.lobby.id == lobby.id
        }) {
            // Asking again while the first invitation is open: tell the inviter where it stands.
            self.deliver_to_session(
                &from.session,
                Message::InviteUpdate(open.update(proto::InviteState::Pending, unix_millis())),
            );
            return ack();
        }
        let open_from_session = self
            .invites
            .values()
            .filter(|invite| invite.from.session == from.session)
            .count();
        if open_from_session >= Self::MAX_OPEN_INVITES_PER_SESSION {
            return Err(Refusal::new(
                ErrorCode::RateLimited,
                "too many open invitations",
            ));
        }
        let lookup = friend.clone();
        let Some(record) = self.db(move |store| store.account(&lookup)).await? else {
            return Err(Refusal::new(ErrorCode::AccountNotFound, "unknown account"));
        };
        let now = unix_millis();
        let lifetime = self.timing.invite_lifetime;
        let invite = Invite {
            id: InviteId::random(),
            from,
            to: record.to_identity(),
            to_account: friend.clone(),
            lobby,
            expires_at_ms: now
                .saturating_add(i64::try_from(lifetime.as_millis()).unwrap_or(i64::MAX)),
        };
        self.deliver_to_session(
            &invite.from.session,
            Message::InviteUpdate(invite.update(proto::InviteState::Pending, now)),
        );
        self.deliver(&friend, Message::GameInvite(invite.to_wire(now)));
        let notification = notifications::invitation(&invite, now, lifetime);
        self.notify_once(
            NotificationKind::Invitation,
            &invite.from.account,
            &friend,
            notification,
        );
        let handle = self.handle.clone();
        let id = invite.id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(lifetime).await;
            handle.send(SocialCommand::InviteExpired(id));
        });
        self.invites.insert(invite.id.clone(), invite);
        ack()
    }

    fn cancel_invite(&mut self, from: &Requester, raw: &str) -> Outcome {
        let invite = InviteId::parse(raw)
            .and_then(|id| self.invites.remove_entry(&id))
            .map(|(_, invite)| invite);
        let Some(invite) = invite else {
            return Err(Refusal::new(
                ErrorCode::InviteClosed,
                "the invitation is no longer open",
            ));
        };
        if invite.from.session != from.session {
            // Not this session's to cancel: put it back.
            self.invites.insert(invite.id.clone(), invite);
            return Err(Refusal::new(
                ErrorCode::InviteClosed,
                "the invitation is no longer open",
            ));
        }
        self.close_invite(&invite, proto::InviteState::Cancelled, None);
        ack()
    }

    fn answer_invite(&mut self, from: &Requester, answer: proto::AnswerInvite) -> Outcome {
        let state = match proto::InviteAnswer::try_from(answer.answer) {
            Ok(proto::InviteAnswer::Accept) => proto::InviteState::Accepted,
            Ok(proto::InviteAnswer::Decline) => proto::InviteState::Declined,
            _ => return Err(Refusal::new(ErrorCode::InvalidRequest, "accept or decline")),
        };
        let open = InviteId::parse(&answer.invite_id)
            .filter(|id| {
                self.invites
                    .get(id)
                    .is_some_and(|invite| invite.to_account == from.account)
            })
            .and_then(|id| self.invites.remove(&id));
        let Some(invite) = open else {
            return Err(Refusal::new(
                ErrorCode::InviteClosed,
                "the invitation is no longer open",
            ));
        };
        self.close_invite(&invite, state, Some(&from.session));
        ack()
    }

    /// Tells the inviter and the invited devices. Only an answer pushes a notification to the
    /// friend's other devices: an expired or cancelled invitation would put a second notification
    /// on every device that ignored the first, so it stays and a tap says it is closed.
    fn close_invite(
        &self,
        invite: &Invite,
        state: proto::InviteState,
        answered_by: Option<&SessionId>,
    ) {
        // Through the hub to the session, not the connection it asked from: that one may have
        // been replaced by a reconnect since.
        self.deliver_to_session(
            &invite.from.session,
            Message::InviteUpdate(invite.update(state, unix_millis())),
        );
        self.deliver(
            &invite.to_account,
            Message::GameInviteClosed(proto::GameInviteClosed {
                invite_id: invite.id.as_str().to_owned(),
                state: state as i32,
            }),
        );
        if let Some(session) = answered_by {
            let answering_device = self.session_targets.get(session).cloned();
            self.notify(
                &invite.to_account,
                notifications::invitation_answered_elsewhere(invite, state, unix_millis()),
                answering_device,
            );
        }
    }

    fn close_invites_from(&mut self, session: &SessionId) {
        let gone: Vec<InviteId> = self
            .invites
            .values()
            .filter(|invite| &invite.from.session == session)
            .map(|invite| invite.id.clone())
            .collect();
        for id in gone {
            if let Some(invite) = self.invites.remove(&id) {
                self.close_invite(&invite, proto::InviteState::Cancelled, None);
            }
        }
    }

    /// Sends `notification` to every device of `account` except `skip` (a push address), on the
    /// push queue. Devices the push service no longer knows are forgotten.
    fn notify(&self, account: &AccountId, notification: PushNotification, skip: Option<String>) {
        let store = self.store.clone();
        let push = self.push.clone();
        let account = account.clone();
        self.push_queue.submit(move || {
            let targets = match store.push_targets(&account) {
                Ok(targets) => targets,
                Err(error) => {
                    tracing::warn!(error = %error_chain(&error), "push targets not read");
                    return;
                }
            };
            for target in targets
                .iter()
                .filter(|target| skip.as_deref() != Some(target.address()))
                .filter(|target| push.can_reach(target))
            {
                match push.send(target, &notification) {
                    Ok(()) => {}
                    Err(PushError::TargetGone) => {
                        if let Err(error) = store.forget_push_target(target.address()) {
                            tracing::warn!(error = %error_chain(&error), "push target not forgotten");
                        }
                    }
                    Err(error) => {
                        tracing::warn!(error = %error_chain(&error), %account, "push not sent");
                    }
                }
            }
        });
    }

    async fn presence_changed(&mut self, account: AccountId, presence: proto::Presence) {
        if self.presences.get(&account) == Some(&presence)
            || (presence == proto::Presence::Offline && !self.presences.contains_key(&account))
        {
            return;
        }
        let owner = account.clone();
        let friends = match self.db(move |store| store.friend_ids(&owner)).await {
            Ok(friends) => friends,
            Err(refusal) => {
                // Not remembered as told: the next report tries again.
                tracing::warn!(message = refusal.message, "presence not shared");
                return;
            }
        };
        if presence == proto::Presence::Offline {
            self.presences.remove(&account);
        } else {
            self.presences.insert(account.clone(), presence);
        }
        let update = proto::FriendPresence {
            account_id: account.as_str().to_owned(),
            presence: presence as i32,
        };
        for friend in &friends {
            self.deliver(friend, Message::FriendPresence(update.clone()));
        }
    }
}
