use std::collections::BTreeSet;

use mg_core::Clock;
use mg_protocol::proto;
use mg_protocol::proto::server_message::Message;
use mg_registry::{SeatSpec, create_authority, load_replay_file};
use tokio::sync::mpsc;

use super::hub::Hub;
use super::lobby::StoredReplay;
use super::lobby_phase::LobbyPhase;
use super::lobby_seat::LobbySeat;
use crate::codec;
use crate::context::SERVER_VERSION;
use crate::ids::{LobbyId, MatchId, SessionId};
use crate::limits::Limits;
use crate::matches::{
    MatchActor, MatchCommand, MatchFinished, MatchHandle, MatchSetup, Participant,
};

use proto::ErrorCode;

/// Commands queued per match actor; a turn-based actor busy with a bot search may fall behind.
const MATCH_QUEUE: usize = 256;

impl Hub {
    pub(super) fn spawn_match(&mut self, lobby_id: &LobbyId) -> Result<(), String> {
        let Some(lobby) = self.lobbies.get(lobby_id) else {
            return Err("lobby not found".into());
        };
        let info = lobby.kind.info();
        if lobby.seats.len() < usize::from(info.min_players) {
            return Err(format!(
                "{} needs at least {} players",
                info.name, info.min_players
            ));
        }
        let seat_specs: Vec<SeatSpec> = lobby
            .seats
            .iter()
            .map(|seat| match seat {
                LobbySeat::Human { .. } => SeatSpec::Human,
                LobbySeat::Bot { kind, .. } => SeatSpec::Bot(*kind),
            })
            .collect();
        let authority = create_authority(&lobby.settings, &seat_specs, rand::random())
            .map_err(|e| e.to_string())?;
        let match_id = MatchId::random();
        let tick_ms = match authority.clock() {
            Clock::Realtime { tick_ms } => tick_ms,
            Clock::TurnBased => 0,
        };
        let seats = lobby
            .seats
            .iter()
            .enumerate()
            .map(|(slot, seat)| proto::MatchSeat {
                slot: u32::try_from(slot).unwrap_or(u32::MAX),
                player: Some(match seat {
                    LobbySeat::Human { session, .. } => self.identity_of(session),
                    LobbySeat::Bot { identity, .. } => identity.clone(),
                }),
                bot_kind: match seat {
                    LobbySeat::Bot { kind, .. } => Some(*kind),
                    LobbySeat::Human { .. } => None,
                },
            })
            .collect();
        let header = proto::MatchHeader {
            match_id: match_id.to_string(),
            settings: Some(lobby.settings),
            seats,
            tick_ms,
            engine_version: SERVER_VERSION.to_owned(),
            started_at_unix_ms: crate::clock::unix_millis(),
        };
        let seated = lobby.seats.iter().enumerate().filter_map(|(slot, seat)| {
            let slot = mg_core::Slot::try_from(slot).ok()?;
            seat.session().map(|session| (session, Some(slot)))
        });
        let watching = lobby.observers.iter().map(|session| (session, None));
        let participants = seated
            .chain(watching)
            .map(|(session, slot)| Participant {
                session: session.clone(),
                slot,
                outbound: self
                    .sessions
                    .get(session)
                    .and_then(|s| s.outbound().cloned()),
            })
            .collect();

        let (sender, receiver) = mpsc::channel(MATCH_QUEUE);
        let actor = MatchActor::new(MatchSetup {
            lobby: lobby_id.clone(),
            match_id: match_id.clone(),
            header,
            authority,
            participants,
            hub: self.handle.clone(),
            bot_cpu: self.bot_cpu.clone(),
        });
        tokio::spawn(actor.run(receiver));
        if let Some(lobby) = self.lobbies.get_mut(lobby_id) {
            lobby.phase = LobbyPhase::InMatch(MatchHandle::new(match_id, sender));
            lobby.reset_ready();
        }
        self.broadcast_lobby_state(lobby_id);
        self.broadcast_lobby_list();
        Ok(())
    }

    fn match_of(&self, session: &SessionId, match_id: &str) -> Option<MatchHandle> {
        let lobby_id = self.sessions.get(session)?.lobby.clone()?;
        let handle = self.lobbies.get(&lobby_id)?.phase.match_handle()?;
        (handle.match_id.as_str() == match_id).then(|| handle.clone())
    }

    pub(super) fn forward_to_match(
        &self,
        session: &SessionId,
        request_id: u32,
        match_id: &str,
        command: MatchCommand,
    ) {
        match self.match_of(session, match_id) {
            Some(handle) => handle.send(command),
            None => self.reject(
                session,
                request_id,
                ErrorCode::MatchNotFound,
                "no such running match",
            ),
        }
    }

    pub(super) fn submit_command(
        &self,
        session: &SessionId,
        request_id: u32,
        submit: proto::SubmitCommand,
    ) {
        let Some(command) = submit.command else {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidRequest,
                "command is required",
            );
        };
        let command = MatchCommand::Submit {
            session: session.clone(),
            request_id,
            command,
        };
        self.forward_to_match(session, request_id, &submit.match_id, command);
    }

    pub(super) fn match_finished(&mut self, finished: MatchFinished) {
        let Some(lobby) = self.lobbies.get_mut(&finished.lobby) else {
            return;
        };
        if lobby
            .phase
            .match_handle()
            .is_none_or(|h| h.match_id != finished.match_id)
        {
            return;
        }
        tracing::info!(
            lobby = %finished.lobby,
            match_id = %finished.match_id,
            outcome = finished.result.kind,
            "match finished"
        );
        lobby.phase = LobbyPhase::PostMatch {
            play_again_votes: BTreeSet::new(),
        };
        lobby.store_replay(StoredReplay {
            match_id: finished.match_id,
            file: finished.replay,
        });
        lobby.last_activity = std::time::Instant::now();
        self.broadcast_lobby_state(&finished.lobby);
        self.broadcast_lobby_list();
    }

    pub(super) fn request_replay(&self, session: &SessionId, request_id: u32, match_id: &str) {
        let replay = self
            .sessions
            .get(session)
            .and_then(|s| s.lobby.as_ref())
            .and_then(|lobby_id| self.lobbies.get(lobby_id))
            .and_then(|lobby| {
                lobby
                    .replays
                    .iter()
                    .find(|r| r.match_id.as_str() == match_id)
            });
        let Some(replay) = replay else {
            return self.reject(
                session,
                request_id,
                ErrorCode::MatchNotFound,
                "replay not available",
            );
        };
        let message = Message::ReplayData(proto::ReplayData {
            match_id: replay.match_id.to_string(),
            replay_file: replay.file.to_vec(),
            suggested_file_name: format!("match-{}.mgreplay", replay.match_id),
        });
        self.send_to(session, &codec::reply(request_id, message));
    }

    /// Validates an uploaded replay just enough to know every viewer can play it; playback is
    /// client-side, so the server never simulates untrusted replays.
    pub(super) fn share_replay(
        &mut self,
        session: &SessionId,
        request_id: u32,
        share: proto::ShareReplay,
    ) {
        let Some(lobby_id) = self.sessions.get(session).and_then(|s| s.lobby.clone()) else {
            return self.reject(
                session,
                request_id,
                ErrorCode::NotInLobby,
                "you are not in a lobby",
            );
        };
        let Some(lobby) = self.lobbies.get(&lobby_id) else {
            return;
        };
        if &lobby.host != session {
            return self.reject(
                session,
                request_id,
                ErrorCode::NotHost,
                "only the host can share a replay",
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
        if share.replay_file.len() > Limits::MAX_REPLAY_BYTES {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidReplay,
                "replay is too large",
            );
        }
        if let Err(error) = load_replay_file(&share.replay_file) {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidReplay,
                &error.to_string(),
            );
        }
        let Some(lobby) = self.lobbies.get_mut(&lobby_id) else {
            return;
        };
        lobby.phase = LobbyPhase::Replay {
            host_only_control: share.host_only_control,
        };
        let message = codec::push(Message::SharedReplay(proto::SharedReplay {
            replay_file: share.replay_file,
            host_only_control: share.host_only_control,
        }));
        let encoded = codec::encode(&message);
        for member in lobby.members() {
            if let Some(outbound) = self.sessions.get(member).and_then(|s| s.outbound()) {
                outbound.send_encoded(encoded.clone());
            }
        }
        self.ack(session, request_id);
        self.broadcast_lobby_state(&lobby_id);
        self.broadcast_lobby_list();
    }

    pub(super) fn replay_control(
        &mut self,
        session: &SessionId,
        request_id: u32,
        control: proto::ReplayControl,
    ) {
        let Some(lobby_id) = self.sessions.get(session).and_then(|s| s.lobby.clone()) else {
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
        let LobbyPhase::Replay { host_only_control } = lobby.phase else {
            return self.reject(
                session,
                request_id,
                ErrorCode::WrongPhase,
                "no shared replay",
            );
        };
        if host_only_control && &lobby.host != session {
            return self.reject(
                session,
                request_id,
                ErrorCode::NotHost,
                "only the host controls this replay",
            );
        }
        let stop = control.action == proto::ReplayControlAction::Stop as i32;
        if stop {
            lobby.phase = LobbyPhase::Open;
        }
        let encoded = codec::encode(&codec::push(Message::ReplayControl(control)));
        for member in lobby.members() {
            if let Some(outbound) = self.sessions.get(member).and_then(|s| s.outbound()) {
                outbound.send_encoded(encoded.clone());
            }
        }
        self.ack(session, request_id);
        if stop {
            self.broadcast_lobby_state(&lobby_id);
            self.broadcast_lobby_list();
        }
    }
}
