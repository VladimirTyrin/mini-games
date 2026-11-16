use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as ClientPayload;
use mg_protocol::proto::server_message::Message;
use tokio::sync::{Semaphore, mpsc};
use tokio::time::Instant;

use super::hub_command::{HubCommand, SignIn};
use super::hub_handle::HubHandle;
use super::lobby::Lobby;
use super::lobby_phase::LobbyPhase;
use super::lobby_seat::LobbySeat;
use super::request_cooldowns::RequestCooldowns;
use super::session::{Session, SessionConnection};
use crate::authenticated::Authenticated;
use crate::codec;
use crate::ids::{ConnectionId, LobbyId, ResumeToken, SessionId};
use crate::matches::{MatchControl, MatchRequest};
use crate::outbound::Outbound;
use crate::player_id::PlayerId;
use crate::timing::Timing;

/// Owns every session and lobby. Runs as one task and handles one command at a time, so lobby
/// state needs no locks; anything slow (bots, broadcasts of frames, replay validation) runs
/// elsewhere.
pub(crate) struct Hub {
    pub(super) handle: HubHandle,
    pub(super) bot_cpu: Arc<Semaphore>,
    pub(super) timing: Timing,
    pub(super) sessions: HashMap<SessionId, Session>,
    /// Exclusive account (a bot key) → session, so a second sign-in takes the session over.
    /// Display names are labels only and may repeat; players are told apart by id.
    pub(super) accounts: HashMap<PlayerId, SessionId>,
    pub(super) resume_tokens: HashMap<ResumeToken, SessionId>,
    /// BTreeMap keeps the lobby list order stable.
    pub(super) lobbies: BTreeMap<LobbyId, Lobby>,
    /// A `FlushLobbyList` is scheduled; changes until then are coalesced into one push.
    lobby_list_flush_scheduled: bool,
    /// What browsers last received, so a flush without visible changes pushes nothing.
    last_pushed_lobby_list: Option<proto::LobbyList>,
}

impl Hub {
    pub(crate) fn new(handle: HubHandle, bot_cpu: Arc<Semaphore>, timing: Timing) -> Self {
        Self {
            handle,
            bot_cpu,
            timing,
            sessions: HashMap::new(),
            accounts: HashMap::new(),
            resume_tokens: HashMap::new(),
            lobbies: BTreeMap::new(),
            lobby_list_flush_scheduled: false,
            last_pushed_lobby_list: None,
        }
    }

    pub(crate) async fn run(mut self, mut commands: mpsc::UnboundedReceiver<HubCommand>) {
        while let Some(command) = commands.recv().await {
            match command {
                HubCommand::SignIn(sign_in) => self.sign_in(sign_in),
                HubCommand::Client {
                    session,
                    connection,
                    request_id,
                    message,
                } => {
                    if self
                        .sessions
                        .get(&session)
                        .is_some_and(|s| s.is_connection(connection))
                    {
                        self.handle_client(&session, request_id, message);
                    }
                }
                HubCommand::Disconnected {
                    session,
                    connection,
                } => self.disconnected(&session, connection),
                HubCommand::SignOut {
                    session,
                    connection,
                } => {
                    if self
                        .sessions
                        .get(&session)
                        .is_some_and(|s| s.is_connection(connection))
                    {
                        self.end_session(&session);
                    }
                }
                HubCommand::MatchFinished(finished) => self.match_finished(finished),
                HubCommand::MatchAborted(aborted) => self.match_aborted(aborted),
                HubCommand::ReplayValidated(validated) => self.replay_validated(validated),
                HubCommand::ResumeGraceExpired {
                    session,
                    connection,
                } => self.resume_grace_expired(&session, connection),
                HubCommand::FlushLobbyList => self.flush_lobby_list(),
                HubCommand::Housekeeping => self.housekeeping(),
                HubCommand::Shutdown => {
                    self.shutdown();
                    return;
                }
            }
        }
    }

    fn sign_in(&mut self, sign_in: SignIn) {
        let SignIn {
            connection,
            outbound,
            who,
            session_token,
            resume,
            server_info,
            request_id,
            reply,
        } = sign_in;
        let connection_info = SessionConnection {
            id: connection,
            outbound: outbound.clone(),
        };

        let resumable = resume
            .as_ref()
            .and_then(|token| self.resume_tokens.get(token))
            .filter(|id| {
                self.sessions
                    .get(*id)
                    .is_some_and(|s| s.who.is_same_player(&who))
            })
            .cloned();
        let existing = resumable
            .or_else(|| {
                who.exclusive_account()
                    .and_then(|key| self.accounts.get(key).cloned())
            })
            .or_else(|| self.away_device_session(&who));

        let created = existing.is_none();
        let session_id = if let Some(id) = existing {
            let Some(session) = self.sessions.get_mut(&id) else {
                return;
            };
            if let Some(previous) = session.connection.replace(connection_info) {
                // The same session from a second connection (a bot restarted, a duplicated tab
                // with the same resume token): the old connection loses.
                previous.outbound.send(&codec::error(
                    0,
                    proto::ErrorCode::SignedInElsewhere,
                    "signed in from another connection",
                ));
                previous.outbound.close();
            }
            if session.who != who {
                // An account may come back with a new profile name or picture; lobby members
                // see them in the `LobbyState` that `rejoin_lobby` pushes below.
                session.identity = who.to_identity(&id);
                session.who = who;
            }
            id
        } else {
            let id = SessionId::random();
            let identity = who.to_identity(&id);
            let resume_token = ResumeToken::random();
            if let Some(account) = who.exclusive_account() {
                self.accounts.insert(account.clone(), id.clone());
            }
            self.resume_tokens.insert(resume_token.clone(), id.clone());
            self.sessions.insert(
                id.clone(),
                Session {
                    who,
                    identity,
                    resume_token,
                    connection: Some(connection_info),
                    lost_connection: None,
                    lobby: None,
                    cooldowns: RequestCooldowns::default(),
                    validating_replay: false,
                },
            );
            id
        };

        let Some(session) = self.sessions.get(&session_id) else {
            return;
        };
        let lobby = session.lobby.clone();
        outbound.send(&codec::reply(
            request_id,
            Message::Welcome(proto::Welcome {
                identity: Some(session.identity.clone()),
                session_token,
                resume_token: session.resume_token.to_string(),
                server_info: Some(server_info),
                resumed: lobby.is_some(),
            }),
        ));
        if reply.send(Some(session_id.clone())).is_err() {
            // The connection went away while signing in; nobody will ever disconnect it.
            if created {
                self.end_session(&session_id);
            } else {
                self.disconnected(&session_id, connection);
            }
            return;
        }
        if let Some(lobby_id) = lobby {
            self.rejoin_lobby(&session_id, &lobby_id, outbound);
        }
    }

    /// A device of this Google account that lost its connection: a sign-in without its resume
    /// token (a closed tab, an app the OS killed, a dead phone and the laptop instead) takes it
    /// back, seat included, rather than leaving it to time out. Connected devices stay separate
    /// players. One in a lobby first, since that is a seat to get back.
    fn away_device_session(&self, who: &Authenticated) -> Option<SessionId> {
        let account = who.google_account()?;
        let mut away = self.sessions.iter().filter(|(_, session)| {
            session.connection.is_none() && session.who.google_account() == Some(account)
        });
        let first = away.clone().next().map(|(id, _)| id.clone());
        away.find(|(_, session)| session.lobby.is_some())
            .map(|(id, _)| id.clone())
            .or(first)
    }

    fn rejoin_lobby(&mut self, session: &SessionId, lobby_id: &LobbyId, outbound: Outbound) {
        self.broadcast_lobby_state(lobby_id);
        let Some(lobby) = self.lobbies.get(lobby_id) else {
            return;
        };
        match &lobby.phase {
            LobbyPhase::InMatch(handle) => handle.control(MatchControl::Attach {
                session: session.clone(),
                outbound,
            }),
            LobbyPhase::Replay(shared) => Self::send_shared_replay(shared, &outbound),
            LobbyPhase::Open | LobbyPhase::PostMatch { .. } => {}
        }
    }

    fn disconnected(&mut self, session_id: &SessionId, connection: ConnectionId) {
        let Some(session) = self.sessions.get_mut(session_id) else {
            return;
        };
        if !session.is_connection(connection) {
            return;
        }
        session.connection = None;
        session.lost_connection = Some(connection);
        if let Some(lobby_id) = session.lobby.clone() {
            if let Some(handle) = self
                .lobbies
                .get(&lobby_id)
                .and_then(|l| l.phase.match_handle())
            {
                handle.control(MatchControl::Detach {
                    session: session_id.clone(),
                });
            }
            self.broadcast_lobby_state(&lobby_id);
        }
        let handle = self.handle.clone();
        let session = session_id.clone();
        let grace = self.timing.resume_grace;
        tokio::spawn(async move {
            tokio::time::sleep(grace).await;
            handle.send(HubCommand::ResumeGraceExpired {
                session,
                connection,
            });
        });
    }

    fn resume_grace_expired(&mut self, session_id: &SessionId, connection: ConnectionId) {
        // A reconnect followed by another disconnect arms a newer timer; only that one may act.
        let still_gone = self
            .sessions
            .get(session_id)
            .is_some_and(|s| s.connection.is_none() && s.lost_connection == Some(connection));
        if still_gone {
            self.end_session(session_id);
        }
    }

    pub(super) fn end_session(&mut self, session_id: &SessionId) {
        self.leave_lobby(session_id);
        if let Some(session) = self.sessions.remove(session_id) {
            if let Some(account) = session.who.exclusive_account() {
                self.accounts.remove(account);
            }
            self.resume_tokens.remove(&session.resume_token);
        }
    }

    fn handle_client(&mut self, session: &SessionId, request_id: u32, message: ClientPayload) {
        match message {
            ClientPayload::ListLobbies(_) => self.send_lobby_list(session, request_id),
            ClientPayload::CreateLobby(create) => self.create_lobby(session, request_id, create),
            ClientPayload::JoinLobby(join) => self.join_lobby(session, request_id, join),
            ClientPayload::LeaveLobby(_) | ClientPayload::LeaveMatch(_) => {
                self.leave_lobby(session);
                self.ack(session, request_id);
            }
            ClientPayload::SetReady(ready) => self.set_ready(session, request_id, ready.ready),
            ClientPayload::UpdateLobbySettings(update) => {
                self.update_settings(session, request_id, update)
            }
            ClientPayload::AddBot(add) => self.add_bot(session, request_id, add),
            ClientPayload::RemoveSeat(remove) => self.remove_seat(session, request_id, remove.seat),
            ClientPayload::KickMember(kick) => self.kick_member(session, request_id, kick),
            ClientPayload::SwitchToObserver(_) => self.switch_to_observer(session, request_id),
            ClientPayload::SwitchToPlayer(_) => self.switch_to_player(session, request_id),
            ClientPayload::StartMatch(_) => self.start_match(session, request_id),
            ClientPayload::VotePlayAgain(_) => self.vote_play_again(session, request_id),
            ClientPayload::SendChat(chat) => self.send_chat(session, request_id, chat),
            ClientPayload::SubmitCommand(submit) => {
                self.submit_command(session, request_id, submit)
            }
            ClientPayload::ReportDesync(desync) => {
                if self.try_catch_up(session, request_id) {
                    tracing::warn!(%session, match_id = desync.match_id, tick = desync.tick, "client reported desync");
                    let request = MatchRequest::CatchUp {
                        session: session.clone(),
                    };
                    self.forward_to_match(session, request_id, &desync.match_id, request);
                }
            }
            ClientPayload::RequestCatchUp(catch_up) => {
                if self.try_catch_up(session, request_id) {
                    let request = MatchRequest::CatchUp {
                        session: session.clone(),
                    };
                    self.forward_to_match(session, request_id, &catch_up.match_id, request);
                }
            }
            ClientPayload::VoteDropPlayer(vote) => {
                let request = MatchRequest::VoteDrop {
                    session: session.clone(),
                    request_id,
                    slot: vote.slot,
                };
                self.forward_to_match(session, request_id, &vote.match_id, request);
            }
            ClientPayload::RequestReplay(request) => {
                self.request_replay(session, request_id, &request.match_id)
            }
            ClientPayload::ShareReplay(share) => self.share_replay(session, request_id, share),
            ClientPayload::ReplayControl(control) => {
                self.replay_control(session, request_id, control)
            }
            // Handled by the connection before the hub sees anything.
            ClientPayload::ServerInfo(_)
            | ClientPayload::Hello(_)
            | ClientPayload::SignOut(_)
            | ClientPayload::Ping(_)
            | ClientPayload::RequestTestPush(_) => {}
        }
    }

    /// Applies the per-session catch-up cooldown; replies `RATE_LIMITED` when it is not over.
    fn try_catch_up(&mut self, session: &SessionId, request_id: u32) -> bool {
        let allowed = self
            .sessions
            .get_mut(session)
            .is_some_and(|s| s.cooldowns.try_catch_up(Instant::now()));
        if !allowed {
            self.reject(
                session,
                request_id,
                proto::ErrorCode::RateLimited,
                "catch-up was just sent; try again shortly",
            );
        }
        allowed
    }

    pub(super) fn ack(&self, session: &SessionId, request_id: u32) {
        self.send_to(
            session,
            &codec::reply(request_id, Message::Ack(proto::Ack {})),
        );
    }

    pub(super) fn reject(
        &self,
        session: &SessionId,
        request_id: u32,
        code: proto::ErrorCode,
        text: &str,
    ) {
        self.send_to(session, &codec::error(request_id, code, text));
    }

    pub(super) fn send_to(&self, session: &SessionId, message: &proto::ServerMessage) {
        if let Some(session) = self.sessions.get(session) {
            session.send(message);
        }
    }

    pub(super) fn identity_of(&self, session: &SessionId) -> proto::Identity {
        self.sessions
            .get(session)
            .map(|s| s.identity.clone())
            .unwrap_or_default()
    }

    pub(super) fn lobby_state(&self, lobby: &Lobby) -> proto::LobbyState {
        let seats = lobby
            .seats
            .iter()
            .map(|seat| match seat {
                LobbySeat::Human { session, ready } => proto::LobbySeat {
                    occupant: Some(self.identity_of(session)),
                    bot_kind: None,
                    ready: *ready,
                    connection_lost: self
                        .sessions
                        .get(session)
                        .is_some_and(|s| s.connection.is_none()),
                },
                LobbySeat::Bot { kind, identity } => proto::LobbySeat {
                    occupant: Some(identity.clone()),
                    bot_kind: Some(*kind),
                    ready: true,
                    connection_lost: false,
                },
            })
            .collect();
        let (play_again_votes, last_match) = match &lobby.phase {
            LobbyPhase::PostMatch { play_again_votes } => (
                play_again_votes
                    .iter()
                    .map(|s| self.identity_of(s))
                    .collect(),
                lobby.last_match.as_ref(),
            ),
            _ => (Vec::new(), None),
        };
        proto::LobbyState {
            lobby_id: lobby.id.to_string(),
            name: lobby.name.clone(),
            host: Some(self.identity_of(&lobby.host)),
            settings: Some(lobby.settings),
            max_seats: u32::from(lobby.max_seats),
            seats,
            observers: lobby
                .observers
                .iter()
                .map(|s| self.identity_of(s))
                .collect(),
            phase: lobby.phase.to_wire() as i32,
            play_again_votes,
            last_match_id: last_match
                .map(|finished| finished.match_id.to_string())
                .unwrap_or_default(),
            last_result: last_match.map(|finished| finished.result.clone()),
        }
    }

    pub(super) fn broadcast_lobby_state(&self, lobby_id: &LobbyId) {
        let Some(lobby) = self.lobbies.get(lobby_id) else {
            return;
        };
        let encoded = codec::encode(&codec::push(Message::LobbyState(self.lobby_state(lobby))));
        self.send_to_members(lobby, &encoded);
    }

    pub(super) fn send_to_members(&self, lobby: &Lobby, encoded: &axum::extract::ws::Utf8Bytes) {
        for member in lobby.members() {
            if let Some(outbound) = self.sessions.get(member).and_then(Session::outbound) {
                outbound.send_encoded(encoded.clone());
            }
        }
    }

    fn lobby_list(&self) -> proto::LobbyList {
        let lobbies = self
            .lobbies
            .values()
            .map(|lobby| proto::LobbySummary {
                lobby_id: lobby.id.to_string(),
                name: lobby.name.clone(),
                settings: Some(lobby.settings),
                seats_taken: u32::try_from(lobby.seats.len()).unwrap_or(u32::MAX),
                max_seats: u32::from(lobby.max_seats),
                observers: u32::try_from(lobby.observers.len()).unwrap_or(u32::MAX),
                phase: lobby.phase.to_wire() as i32,
                host: Some(self.identity_of(&lobby.host)),
            })
            .collect();
        proto::LobbyList { lobbies }
    }

    fn send_lobby_list(&self, session: &SessionId, request_id: u32) {
        self.send_to(
            session,
            &codec::reply(request_id, Message::LobbyList(self.lobby_list())),
        );
    }

    pub(super) fn send_lobby_list_push(&self, session: &SessionId) {
        self.send_to(session, &codec::push(Message::LobbyList(self.lobby_list())));
    }

    /// Something in some lobby changed. Browsers get at most one list push per coalescing
    /// window, and only when a summary they see actually differs.
    pub(super) fn lobby_list_changed(&mut self) {
        if self.lobby_list_flush_scheduled {
            return;
        }
        self.lobby_list_flush_scheduled = true;
        let handle = self.handle.clone();
        let delay = self.timing.lobby_list_coalesce;
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            handle.send(HubCommand::FlushLobbyList);
        });
    }

    /// Pushes the list to everyone browsing lobbies (signed in and not inside a lobby).
    fn flush_lobby_list(&mut self) {
        self.lobby_list_flush_scheduled = false;
        let list = self.lobby_list();
        if self.last_pushed_lobby_list.as_ref() == Some(&list) {
            return;
        }
        let encoded = codec::encode(&codec::push(Message::LobbyList(list.clone())));
        for session in self.sessions.values().filter(|s| s.lobby.is_none()) {
            if let Some(outbound) = session.outbound() {
                outbound.send_encoded(encoded.clone());
            }
        }
        self.last_pushed_lobby_list = Some(list);
    }

    fn housekeeping(&mut self) {
        let now = Instant::now();
        let idle: Vec<LobbyId> = self
            .lobbies
            .values()
            .filter(|lobby| {
                lobby.phase.match_handle().is_none()
                    && now.saturating_duration_since(lobby.last_activity)
                        > self.timing.idle_lobby_timeout
            })
            .map(|lobby| lobby.id.clone())
            .collect();
        for lobby_id in idle {
            self.close_lobby(&lobby_id, proto::LobbyClosedReason::Inactive);
        }
    }

    fn shutdown(&mut self) {
        let encoded = codec::encode(&codec::push(Message::ShuttingDown(
            proto::ServerShuttingDown {},
        )));
        for lobby in self.lobbies.values() {
            if let Some(handle) = lobby.phase.match_handle() {
                handle.control(MatchControl::Stop);
            }
        }
        for session in self.sessions.values() {
            if let Some(outbound) = session.outbound() {
                outbound.send_encoded(encoded.clone());
                outbound.close();
            }
        }
    }
}
