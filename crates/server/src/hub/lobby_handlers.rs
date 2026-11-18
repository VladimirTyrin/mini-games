use std::collections::VecDeque;

use mg_core::PlayerSpec;
use mg_protocol::proto;
use mg_protocol::proto::server_message::Message;
use mg_registry::{GameKind, create_mirror, validate_bot_kind};
use tokio::time::Instant;

use super::hub::Hub;
use super::lobby::Lobby;
use super::lobby_phase::LobbyPhase;
use super::lobby_seat::LobbySeat;
use crate::codec;
use crate::ids::{LobbyId, SessionId};
use crate::limits::Limits;
use crate::matches::MatchControl;
use crate::text;

use proto::ErrorCode;

/// A lobby configuration that passed validation.
struct LobbySetup {
    kind: GameKind,
    max_seats: u8,
}

/// Why lobby settings were refused; the text is shown to the host.
struct SettingsRejection(String);

/// Builds a throwaway game with the minimum seat count, so every setting is checked by the
/// game's own rules. `requested_seats == 0` means "as many as the game allows".
fn validate_lobby_setup(
    settings: &proto::GameSettings,
    requested_seats: u32,
) -> Result<LobbySetup, SettingsRejection> {
    let kind = GameKind::of_settings(settings).map_err(|e| SettingsRejection(e.to_string()))?;
    let info = kind.info();
    let max_seats = if requested_seats == 0 {
        u32::from(info.max_players)
    } else {
        requested_seats
    };
    let max_seats = u8::try_from(max_seats)
        .ok()
        .filter(|seats| (info.min_players..=info.max_players).contains(seats))
        .ok_or_else(|| {
            SettingsRejection(format!(
                "{} supports {}..={} players",
                info.name, info.min_players, info.max_players
            ))
        })?;
    let players = vec![PlayerSpec::HUMAN; usize::from(info.min_players)];
    create_mirror(settings, &players).map_err(|e| SettingsRejection(e.to_string()))?;
    Ok(LobbySetup { kind, max_seats })
}

fn clean_lobby_name(raw: &str, fallback: &str) -> String {
    match text::clean(raw, Limits::MAX_LOBBY_NAME_CHARS) {
        name if name.is_empty() => text::clean(fallback, Limits::MAX_LOBBY_NAME_CHARS),
        name => name,
    }
}

impl Hub {
    fn current_lobby(&self, session: &SessionId) -> Option<LobbyId> {
        self.sessions.get(session).and_then(|s| s.lobby.clone())
    }

    /// Lobby of `session` if it is the host; otherwise replies with an error.
    fn hosted_lobby(&self, session: &SessionId, request_id: u32) -> Option<LobbyId> {
        let Some(lobby_id) = self.current_lobby(session) else {
            self.reject(
                session,
                request_id,
                ErrorCode::NotInLobby,
                "you are not in a lobby",
            );
            return None;
        };
        if self
            .lobbies
            .get(&lobby_id)
            .is_none_or(|l| &l.host != session)
        {
            self.reject(
                session,
                request_id,
                ErrorCode::NotHost,
                "only the host can do that",
            );
            return None;
        }
        Some(lobby_id)
    }

    pub(super) fn touch(&mut self, lobby_id: &LobbyId) {
        if let Some(lobby) = self.lobbies.get_mut(lobby_id) {
            lobby.last_activity = Instant::now();
        }
    }

    pub(super) fn after_lobby_change(&mut self, lobby_id: &LobbyId) {
        self.touch(lobby_id);
        self.broadcast_lobby_state(lobby_id);
        self.lobby_list_changed();
    }

    pub(super) fn create_lobby(
        &mut self,
        session: &SessionId,
        request_id: u32,
        create: proto::CreateLobby,
    ) {
        if self.current_lobby(session).is_some() {
            return self.reject(
                session,
                request_id,
                ErrorCode::AlreadyInLobby,
                "leave your current lobby first",
            );
        }
        if self.lobbies.len() >= Limits::MAX_LOBBIES {
            return self.reject(
                session,
                request_id,
                ErrorCode::RateLimited,
                "too many lobbies on the server",
            );
        }
        let Some(settings) = create.settings else {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidSettings,
                "settings are required",
            );
        };
        let LobbySetup { kind, max_seats } = match validate_lobby_setup(&settings, create.max_seats)
        {
            Ok(setup) => setup,
            Err(SettingsRejection(reason)) => {
                return self.reject(session, request_id, ErrorCode::InvalidSettings, &reason);
            }
        };
        let host_name = self.identity_of(session).display_name;
        let lobby_id = LobbyId::random();
        let lobby = Lobby {
            id: lobby_id.clone(),
            name: clean_lobby_name(&create.name, &format!("{host_name}'s {}", kind.info().name)),
            host: session.clone(),
            kind,
            settings,
            max_seats,
            seats: vec![LobbySeat::Human {
                session: session.clone(),
                ready: false,
            }],
            observers: Vec::new(),
            phase: LobbyPhase::Open,
            last_activity: Instant::now(),
            replays: VecDeque::new(),
            last_match: None,
            next_bot_number: 1,
        };
        self.lobbies.insert(lobby_id.clone(), lobby);
        if let Some(s) = self.sessions.get_mut(session) {
            s.lobby = Some(lobby_id.clone());
        }
        if let Some(lobby) = self.lobbies.get(&lobby_id) {
            self.send_to(
                session,
                &codec::reply(request_id, Message::LobbyState(self.lobby_state(lobby))),
            );
        }
        self.lobby_list_changed();
    }

    pub(super) fn join_lobby(
        &mut self,
        session: &SessionId,
        request_id: u32,
        join: proto::JoinLobby,
    ) {
        if self.current_lobby(session).is_some() {
            return self.reject(
                session,
                request_id,
                ErrorCode::AlreadyInLobby,
                "leave your current lobby first",
            );
        }
        let Some(lobby_id) =
            LobbyId::parse(&join.lobby_id).filter(|id| self.lobbies.contains_key(id))
        else {
            return self.reject(
                session,
                request_id,
                ErrorCode::LobbyNotFound,
                "lobby not found",
            );
        };
        let outbound = self
            .sessions
            .get(session)
            .and_then(|s| s.outbound().cloned());
        let Some(lobby) = self.lobbies.get_mut(&lobby_id) else {
            return;
        };
        let joins_as_player = !join.as_observer && lobby.phase.allows_setup_changes();
        if joins_as_player {
            if lobby.seats.len() >= usize::from(lobby.max_seats) {
                return self.reject(
                    session,
                    request_id,
                    ErrorCode::LobbyFull,
                    "the lobby is full",
                );
            }
            lobby.forget_play_again_votes();
            lobby.seats.push(LobbySeat::Human {
                session: session.clone(),
                ready: false,
            });
        } else {
            if lobby.observers.len() >= Limits::MAX_OBSERVERS {
                return self.reject(
                    session,
                    request_id,
                    ErrorCode::LobbyFull,
                    "too many observers",
                );
            }
            lobby.observers.push(session.clone());
        }
        if let Some(s) = self.sessions.get_mut(session) {
            s.lobby = Some(lobby_id.clone());
        }
        let Some(lobby) = self.lobbies.get(&lobby_id) else {
            return;
        };
        self.send_to(
            session,
            &codec::reply(request_id, Message::LobbyState(self.lobby_state(lobby))),
        );
        // After the reply, so the client knows the lobby before its match or replay arrives.
        if let Some(outbound) = outbound {
            match &lobby.phase {
                LobbyPhase::InMatch(handle) => handle.control(MatchControl::AddObserver {
                    session: session.clone(),
                    outbound,
                }),
                LobbyPhase::Replay(shared) => Self::send_shared_replay(shared, &outbound),
                LobbyPhase::Open | LobbyPhase::PostMatch { .. } => {}
            }
        }
        self.after_lobby_change(&lobby_id);
    }

    /// Leaves the current lobby (and its match, which drops the seat). Hands the host role over,
    /// or closes the lobby when no human is left.
    pub(super) fn leave_lobby(&mut self, session: &SessionId) {
        let Some(lobby_id) = self.current_lobby(session) else {
            return;
        };
        if let Some(s) = self.sessions.get_mut(session) {
            s.lobby = None;
        }
        self.social
            .send(crate::social::SocialCommand::LeftLobby(session.clone()));
        self.announce_presence(session);
        let Some(lobby) = self.lobbies.get_mut(&lobby_id) else {
            return;
        };
        if let Some(handle) = lobby.phase.match_handle() {
            handle.control(MatchControl::Leave {
                session: session.clone(),
            });
        }
        let left_a_seat = lobby.seat_index(session).is_some();
        lobby.remove_member(session);
        // Fewer players is another match (Word Hunt alone is even another mode): the others
        // vote again, knowing who is left, instead of their old votes starting it silently.
        if left_a_seat {
            lobby.forget_play_again_votes();
        }
        let next_host = lobby
            .seated_humans()
            .next()
            .or(lobby.observers.first())
            .cloned();
        match next_host {
            None => self.close_lobby(&lobby_id, proto::LobbyClosedReason::HostLeft),
            Some(next_host) => {
                if &lobby.host == session {
                    lobby.host = next_host;
                    if lobby.phase.allows_setup_changes() {
                        lobby.move_host_seat_first();
                    }
                }
                self.after_lobby_change(&lobby_id);
            }
        }
        self.send_lobby_list_push(session);
    }

    pub(super) fn close_lobby(&mut self, lobby_id: &LobbyId, reason: proto::LobbyClosedReason) {
        let Some(lobby) = self.lobbies.remove(lobby_id) else {
            return;
        };
        if let Some(handle) = lobby.phase.match_handle() {
            handle.control(MatchControl::Stop);
        }
        let closed = codec::push(Message::LobbyClosed(proto::LobbyClosed {
            reason: reason as i32,
        }));
        for member in lobby.members() {
            if let Some(session) = self.sessions.get_mut(member) {
                session.lobby = None;
                session.send(&closed);
            }
            self.social
                .send(crate::social::SocialCommand::LeftLobby(member.clone()));
            self.announce_presence(member);
        }
        self.lobby_list_changed();
    }

    pub(super) fn set_ready(&mut self, session: &SessionId, request_id: u32, ready: bool) {
        let Some(lobby_id) = self.current_lobby(session) else {
            return self.reject(
                session,
                request_id,
                ErrorCode::NotInLobby,
                "you are not in a lobby",
            );
        };
        let Some(lobby) = self.lobbies.get_mut(&lobby_id) else {
            return;
        };
        let Some(LobbySeat::Human {
            ready: seat_ready, ..
        }) = lobby
            .seat_index(session)
            .and_then(|index| lobby.seats.get_mut(index))
        else {
            return self.reject(
                session,
                request_id,
                ErrorCode::NotInLobby,
                "observers have no seat",
            );
        };
        *seat_ready = ready;
        self.ack(session, request_id);
        self.after_lobby_change(&lobby_id);
    }

    pub(super) fn update_settings(
        &mut self,
        session: &SessionId,
        request_id: u32,
        update: proto::UpdateLobbySettings,
    ) {
        let Some(lobby_id) = self.hosted_lobby(session, request_id) else {
            return;
        };
        let Some(settings) = update.settings else {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidSettings,
                "settings are required",
            );
        };
        let LobbySetup { kind, max_seats } = match validate_lobby_setup(&settings, update.max_seats)
        {
            Ok(setup) => setup,
            Err(SettingsRejection(reason)) => {
                return self.reject(session, request_id, ErrorCode::InvalidSettings, &reason);
            }
        };
        let Some(lobby) = self.lobbies.get_mut(&lobby_id) else {
            return;
        };
        if !lobby.phase.allows_setup_changes() {
            return self.reject(
                session,
                request_id,
                ErrorCode::WrongPhase,
                "not during a match",
            );
        }
        if kind != lobby.kind {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidSettings,
                "the game cannot be changed",
            );
        }
        if lobby.seats.len() > usize::from(max_seats) {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidSettings,
                "more players than seats",
            );
        }
        lobby.settings = settings;
        lobby.max_seats = max_seats;
        lobby.reset_ready();
        lobby.forget_play_again_votes();
        self.ack(session, request_id);
        self.after_lobby_change(&lobby_id);
    }

    pub(super) fn add_bot(&mut self, session: &SessionId, request_id: u32, add: proto::AddBot) {
        let Some(lobby_id) = self.hosted_lobby(session, request_id) else {
            return;
        };
        let Some(lobby) = self.lobbies.get_mut(&lobby_id) else {
            return;
        };
        let Some(bot_kind) = add.bot_kind else {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidRequest,
                "bot kind is required",
            );
        };
        if let Err(error) = validate_bot_kind(lobby.kind, &bot_kind) {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidRequest,
                &error.to_string(),
            );
        }
        if !lobby.phase.allows_setup_changes() {
            return self.reject(
                session,
                request_id,
                ErrorCode::WrongPhase,
                "not during a match",
            );
        }
        if lobby.seats.len() >= usize::from(lobby.max_seats) {
            return self.reject(session, request_id, ErrorCode::LobbyFull, "no free seat");
        }
        let number = lobby.next_bot_number;
        lobby.next_bot_number += 1;
        let identity = proto::Identity {
            kind: proto::IdentityKind::BuiltInBot as i32,
            id: format!("bot-{number}"),
            display_name: format!("Bot {number}"),
            avatar_url: String::new(),
        };
        lobby.forget_play_again_votes();
        lobby.seats.push(LobbySeat::Bot {
            kind: bot_kind,
            identity,
        });
        self.ack(session, request_id);
        self.after_lobby_change(&lobby_id);
    }

    pub(super) fn remove_seat(&mut self, session: &SessionId, request_id: u32, seat: u32) {
        let Some(lobby_id) = self.hosted_lobby(session, request_id) else {
            return;
        };
        let Some(lobby) = self.lobbies.get_mut(&lobby_id) else {
            return;
        };
        if !lobby.phase.allows_setup_changes() {
            return self.reject(
                session,
                request_id,
                ErrorCode::WrongPhase,
                "not during a match",
            );
        }
        let index = usize::try_from(seat).unwrap_or(usize::MAX);
        if !matches!(lobby.seats.get(index), Some(LobbySeat::Bot { .. })) {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidRequest,
                "only bot seats can be removed; kick players instead",
            );
        }
        lobby.forget_play_again_votes();
        lobby.seats.remove(index);
        self.ack(session, request_id);
        self.after_lobby_change(&lobby_id);
    }

    pub(super) fn kick_member(
        &mut self,
        session: &SessionId,
        request_id: u32,
        kick: proto::KickMember,
    ) {
        let Some(lobby_id) = self.hosted_lobby(session, request_id) else {
            return;
        };
        let target = self.lobbies.get(&lobby_id).and_then(|lobby| {
            lobby
                .members()
                .find(|m| {
                    self.sessions
                        .get(*m)
                        .is_some_and(|s| s.identity.id == kick.player_id)
                })
                .cloned()
        });
        let Some(target) = target.filter(|t| t != session) else {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidRequest,
                "no such member",
            );
        };
        // Kicking a seated player would forfeit their seat: a host could win that way.
        let plays_in_this_match = self.lobbies.get(&lobby_id).is_some_and(|lobby| {
            matches!(lobby.phase, LobbyPhase::InMatch(_)) && lobby.seat_index(&target).is_some()
        });
        if plays_in_this_match {
            return self.reject(
                session,
                request_id,
                ErrorCode::WrongPhase,
                "players can be kicked only between matches",
            );
        }
        self.leave_lobby(&target);
        self.send_to(
            &target,
            &codec::push(Message::LobbyClosed(proto::LobbyClosed {
                reason: proto::LobbyClosedReason::Kicked as i32,
            })),
        );
        self.ack(session, request_id);
    }

    pub(super) fn switch_to_observer(&mut self, session: &SessionId, request_id: u32) {
        let Some(lobby_id) = self.current_lobby(session) else {
            return self.reject(
                session,
                request_id,
                ErrorCode::NotInLobby,
                "you are not in a lobby",
            );
        };
        let Some(lobby) = self.lobbies.get_mut(&lobby_id) else {
            return;
        };
        if !lobby.phase.allows_setup_changes() {
            return self.reject(
                session,
                request_id,
                ErrorCode::WrongPhase,
                "not during a match",
            );
        }
        let Some(index) = lobby.seat_index(session) else {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidRequest,
                "you are already observing",
            );
        };
        if lobby.observers.len() >= Limits::MAX_OBSERVERS {
            return self.reject(
                session,
                request_id,
                ErrorCode::LobbyFull,
                "too many observers",
            );
        }
        lobby.seats.remove(index);
        lobby.observers.push(session.clone());
        lobby.forget_play_again_votes();
        self.ack(session, request_id);
        self.after_lobby_change(&lobby_id);
    }

    pub(super) fn switch_to_player(&mut self, session: &SessionId, request_id: u32) {
        let Some(lobby_id) = self.current_lobby(session) else {
            return self.reject(
                session,
                request_id,
                ErrorCode::NotInLobby,
                "you are not in a lobby",
            );
        };
        let Some(lobby) = self.lobbies.get_mut(&lobby_id) else {
            return;
        };
        if !lobby.phase.allows_setup_changes() {
            return self.reject(
                session,
                request_id,
                ErrorCode::WrongPhase,
                "not during a match",
            );
        }
        if !lobby.observers.contains(session) {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidRequest,
                "you already have a seat",
            );
        }
        if lobby.seats.len() >= usize::from(lobby.max_seats) {
            return self.reject(session, request_id, ErrorCode::LobbyFull, "no free seat");
        }
        lobby.observers.retain(|o| o != session);
        lobby.forget_play_again_votes();
        lobby.seats.push(LobbySeat::Human {
            session: session.clone(),
            ready: false,
        });
        lobby.move_host_seat_first();
        self.ack(session, request_id);
        self.after_lobby_change(&lobby_id);
    }

    pub(super) fn start_match(&mut self, session: &SessionId, request_id: u32) {
        let Some(lobby_id) = self.hosted_lobby(session, request_id) else {
            return;
        };
        let Some(lobby) = self.lobbies.get(&lobby_id) else {
            return;
        };
        if !lobby.phase.allows_setup_changes() {
            return self.reject(
                session,
                request_id,
                ErrorCode::WrongPhase,
                "a match is already running",
            );
        }
        let everyone_ready = lobby.seats.iter().all(|seat| match seat {
            LobbySeat::Human {
                session: occupant,
                ready,
            } => *ready || occupant == session,
            LobbySeat::Bot { .. } => true,
        });
        if !everyone_ready {
            return self.reject(
                session,
                request_id,
                ErrorCode::NotReady,
                "not everyone is ready",
            );
        }
        match self.spawn_match(&lobby_id) {
            Ok(()) => self.ack(session, request_id),
            Err(error) => self.reject(session, request_id, error.code(), &error.to_string()),
        }
    }

    pub(super) fn vote_play_again(&mut self, session: &SessionId, request_id: u32) {
        let Some(lobby_id) = self.current_lobby(session) else {
            return self.reject(
                session,
                request_id,
                ErrorCode::NotInLobby,
                "you are not in a lobby",
            );
        };
        let Some(lobby) = self.lobbies.get_mut(&lobby_id) else {
            return;
        };
        let LobbyPhase::PostMatch { play_again_votes } = &mut lobby.phase else {
            return self.reject(
                session,
                request_id,
                ErrorCode::WrongPhase,
                "no finished match to replay",
            );
        };
        if lobby
            .seats
            .iter()
            .all(|seat| seat.session() != Some(session))
        {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidRequest,
                "only players vote",
            );
        }
        play_again_votes.insert(session.clone());
        let all_voted = lobby.everyone_voted_to_play_again();
        self.ack(session, request_id);
        if all_voted {
            self.play_again(&lobby_id);
        } else {
            self.after_lobby_change(&lobby_id);
        }
    }

    /// Starts the next match once every seated human voted. A failure (e.g. too few players
    /// left) is told to every member and clears the votes.
    fn play_again(&mut self, lobby_id: &LobbyId) {
        let Err(error) = self.spawn_match(lobby_id) else {
            return;
        };
        tracing::info!(lobby = %lobby_id, %error, "play-again start failed");
        let Some(lobby) = self.lobbies.get_mut(lobby_id) else {
            return;
        };
        if let LobbyPhase::PostMatch { play_again_votes } = &mut lobby.phase {
            play_again_votes.clear();
        }
        let failure = codec::encode(&codec::error(
            0,
            error.code(),
            format!("cannot start the next match: {error}"),
        ));
        if let Some(lobby) = self.lobbies.get(lobby_id) {
            self.send_to_members(lobby, &failure);
        }
        self.after_lobby_change(lobby_id);
    }

    pub(super) fn send_chat(
        &mut self,
        session: &SessionId,
        request_id: u32,
        chat: proto::SendChat,
    ) {
        let text = text::clean(&chat.text, usize::MAX);
        if text.is_empty() || text.chars().count() > Limits::MAX_CHAT_CHARS {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidRequest,
                "message must be 1 to 300 characters",
            );
        }
        let scope = proto::ChatScope::try_from(chat.scope).unwrap_or(proto::ChatScope::Unspecified);
        let message = codec::push(Message::Chat(proto::ChatMessage {
            scope: scope as i32,
            sender: Some(self.identity_of(session)),
            text,
            sent_at_unix_ms: crate::clock::unix_millis(),
        }));
        let encoded = codec::encode(&message);
        match scope {
            proto::ChatScope::LobbyList => {
                for receiver in self.sessions.values().filter(|s| s.lobby.is_none()) {
                    if let Some(outbound) = receiver.outbound() {
                        outbound.send_encoded(encoded.clone());
                    }
                }
            }
            proto::ChatScope::Lobby => {
                let Some(lobby_id) = self.current_lobby(session) else {
                    return self.reject(
                        session,
                        request_id,
                        ErrorCode::NotInLobby,
                        "you are not in a lobby",
                    );
                };
                // A lobby people talk in is in use, even if nobody changes its setup.
                self.touch(&lobby_id);
                let Some(lobby) = self.lobbies.get(&lobby_id) else {
                    return self.reject(
                        session,
                        request_id,
                        ErrorCode::NotInLobby,
                        "you are not in a lobby",
                    );
                };
                for member in lobby.members() {
                    if let Some(outbound) = self.sessions.get(member).and_then(|s| s.outbound()) {
                        outbound.send_encoded(encoded.clone());
                    }
                }
            }
            proto::ChatScope::Unspecified => {
                return self.reject(
                    session,
                    request_id,
                    ErrorCode::InvalidRequest,
                    "chat scope is required",
                );
            }
        }
        self.ack(session, request_id);
    }
}
