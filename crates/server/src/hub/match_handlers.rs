use std::collections::BTreeSet;

use mg_core::{Clock, Tick};
use mg_protocol::proto;
use mg_protocol::proto::server_message::Message;
use mg_registry::{SeatSpec, create_authority, load_replay_file};
use tokio::time::Instant;

use super::finished_match::FinishedMatch;
use super::hub::Hub;
use super::hub_command::HubCommand;
use super::lobby_phase::LobbyPhase;
use super::lobby_seat::LobbySeat;
use super::replay_check_error::ReplayCheckError;
use super::replay_upload::ReplayUpload;
use super::shared_replay::SharedReplay;
use super::spawn_error::SpawnError;
use super::stored_replay::StoredReplay;
use super::validated_replay::ValidatedReplay;
use crate::codec;
use crate::context::server_version;
use crate::ids::{LobbyId, MatchId, SessionId};
use crate::limits::Limits;
use crate::matches::{
    MatchAborted, MatchActor, MatchFinished, MatchHandle, MatchRequest, MatchSetup, Participant,
};
use crate::outbound::Outbound;

use proto::ErrorCode;

const MIN_REPLAY_SPEED_PERCENT: u32 = 25;
const MAX_REPLAY_SPEED_PERCENT: u32 = 400;

impl Hub {
    pub(super) fn spawn_match(&mut self, lobby_id: &LobbyId) -> Result<(), SpawnError> {
        if self.draining {
            return Err(SpawnError::ServerUpdating);
        }
        let Some(lobby) = self.lobbies.get_mut(lobby_id) else {
            return Err(SpawnError::LobbyNotFound);
        };
        // The host may have lost seat 0 mid-match (host hand-over, bots seated before them).
        lobby.move_host_seat_first();
        let info = lobby.kind.info();
        if lobby.seats.len() < usize::from(info.min_players) {
            return Err(SpawnError::NotEnoughPlayers {
                game: info.name,
                min_players: info.min_players,
            });
        }
        let Some(lobby) = self.lobbies.get(lobby_id) else {
            return Err(SpawnError::LobbyNotFound);
        };
        let seat_specs: Vec<SeatSpec> = lobby
            .seats
            .iter()
            .map(|seat| match seat {
                LobbySeat::Human { .. } => SeatSpec::Human,
                LobbySeat::Bot { kind, .. } => SeatSpec::Bot(*kind),
            })
            .collect();
        let authority = create_authority(&lobby.settings, &seat_specs, rand::random())
            .map_err(SpawnError::Create)?;
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
        let people_owe_secrets = lobby.seats.iter().enumerate().any(|(slot, seat)| {
            matches!(seat, LobbySeat::Human { .. })
                && mg_core::Slot::try_from(slot).is_ok_and(|slot| authority.awaits_secret(slot))
        });
        let header = proto::MatchHeader {
            match_id: match_id.to_string(),
            settings: Some(lobby.settings),
            seats,
            tick_ms,
            engine_version: server_version(),
            started_at_unix_ms: crate::clock::unix_millis(),
            secret_setup_ms: if people_owe_secrets {
                u32::try_from(self.timing.secret_setup.as_millis()).unwrap_or(u32::MAX)
            } else {
                0
            },
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

        let (handle, inbox) = MatchHandle::channel(match_id.clone(), Limits::MATCH_REQUEST_QUEUE);
        MatchActor::new(MatchSetup {
            lobby: lobby_id.clone(),
            match_id,
            header,
            authority,
            participants,
            hub: self.handle.clone(),
            bot_cpu: self.bot_cpu.clone(),
            timing: self.timing,
        })
        .spawn(inbox);
        if let Some(lobby) = self.lobbies.get_mut(lobby_id) {
            lobby.phase = LobbyPhase::InMatch(handle);
            lobby.reset_ready();
        }
        self.broadcast_lobby_state(lobby_id);
        self.lobby_list_changed();
        Ok(())
    }

    fn match_of(&self, session: &SessionId, match_id: &str) -> Option<&MatchHandle> {
        let lobby_id = self.sessions.get(session)?.lobby.as_ref()?;
        let handle = self.lobbies.get(lobby_id)?.phase.match_handle()?;
        (handle.match_id.as_str() == match_id).then_some(handle)
    }

    pub(super) fn forward_to_match(
        &self,
        session: &SessionId,
        request_id: u32,
        match_id: &str,
        request: MatchRequest,
    ) {
        let Some(handle) = self.match_of(session, match_id) else {
            return self.reject(
                session,
                request_id,
                ErrorCode::MatchNotFound,
                "no such running match",
            );
        };
        if let Err(busy) = handle.request(request) {
            self.reject(
                session,
                request_id,
                ErrorCode::RateLimited,
                &busy.to_string(),
            );
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
        let request = MatchRequest::Submit {
            session: session.clone(),
            request_id,
            command,
        };
        self.forward_to_match(session, request_id, &submit.match_id, request);
    }

    pub(super) fn submit_secret_command(
        &self,
        session: &SessionId,
        request_id: u32,
        submit: proto::SubmitSecretCommand,
    ) {
        let Some(command) = submit.command else {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidRequest,
                "command is required",
            );
        };
        let request = MatchRequest::SubmitSecret {
            session: session.clone(),
            request_id,
            command,
        };
        self.forward_to_match(session, request_id, &submit.match_id, request);
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
            match_id: finished.match_id.clone(),
            replay_data_json: finished.replay_data_json,
        });
        lobby.last_match = Some(FinishedMatch {
            match_id: finished.match_id,
            result: finished.result,
        });
        lobby.last_activity = Instant::now();
        self.broadcast_lobby_state(&finished.lobby);
        self.lobby_list_changed();
    }

    /// The actor died without a result: members get `Ended` without one and the lobby opens
    /// again, so nobody is stuck in a match that no longer runs.
    pub(super) fn match_aborted(&mut self, aborted: MatchAborted) {
        let Some(lobby) = self.lobbies.get_mut(&aborted.lobby) else {
            return;
        };
        if lobby
            .phase
            .match_handle()
            .is_none_or(|h| h.match_id != aborted.match_id)
        {
            return;
        }
        lobby.phase = LobbyPhase::Open;
        lobby.reset_ready();
        lobby.last_activity = Instant::now();
        let ended = codec::encode(&codec::push(Message::Ended(proto::MatchEnded {
            match_id: aborted.match_id.to_string(),
            result: None,
            final_tick: 0,
        })));
        if let Some(lobby) = self.lobbies.get(&aborted.lobby) {
            self.send_to_members(lobby, &ended);
        }
        self.broadcast_lobby_state(&aborted.lobby);
        self.lobby_list_changed();
    }

    pub(super) fn request_replay(&mut self, session: &SessionId, request_id: u32, match_id: &str) {
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
            })
            .map(|replay| replay.replay_data_json.clone());
        let Some(replay_data_json) = replay else {
            return self.reject(
                session,
                request_id,
                ErrorCode::MatchNotFound,
                "replay not available",
            );
        };
        let Some(session_state) = self.sessions.get_mut(session) else {
            return;
        };
        if !session_state.cooldowns.try_replay_download(Instant::now()) {
            return self.reject(
                session,
                request_id,
                ErrorCode::RateLimited,
                "a replay was just sent; try again shortly",
            );
        }
        if let Some(outbound) = session_state.outbound() {
            outbound.send_encoded(codec::reply_with_encoded_payload(
                request_id,
                "replayData",
                &replay_data_json,
            ));
        }
    }

    /// Checks who may share, then validates the file on the blocking pool (one upload per
    /// session at a time); `replay_validated` finishes the job. Playback is client-side, so the
    /// server never simulates untrusted replays.
    pub(super) fn share_replay(
        &mut self,
        session: &SessionId,
        request_id: u32,
        share: proto::ShareReplay,
    ) {
        let Some(lobby_id) = self.may_share_replay(session, request_id) else {
            return;
        };
        if share.replay_file.len() > Limits::MAX_REPLAY_BYTES {
            return self.reject(
                session,
                request_id,
                ErrorCode::InvalidReplay,
                "replay is too large",
            );
        }
        let Some(session_state) = self.sessions.get_mut(session) else {
            return;
        };
        if session_state.validating_replay {
            return self.reject(
                session,
                request_id,
                ErrorCode::RateLimited,
                "the previous replay is still being checked",
            );
        }
        session_state.validating_replay = true;
        let hub = self.handle.clone();
        let session = session.clone();
        let host_only_control = share.host_only_control;
        tokio::spawn(async move {
            let outcome = match tokio::task::spawn_blocking(move || validate_replay(share)).await {
                Ok(checked) => checked.map_err(ReplayCheckError::Invalid),
                Err(error) => {
                    tracing::error!(%error, "replay validation task failed");
                    Err(ReplayCheckError::TaskFailed)
                }
            };
            hub.send(HubCommand::ReplayValidated(ValidatedReplay {
                session,
                lobby: lobby_id,
                request_id,
                host_only_control,
                outcome,
            }));
        });
    }

    /// The sender's lobby if they host it and it is between matches; otherwise replies with
    /// the reason.
    fn may_share_replay(&self, session: &SessionId, request_id: u32) -> Option<LobbyId> {
        let Some(lobby_id) = self.sessions.get(session).and_then(|s| s.lobby.clone()) else {
            self.reject(
                session,
                request_id,
                ErrorCode::NotInLobby,
                "you are not in a lobby",
            );
            return None;
        };
        let Some(lobby) = self.lobbies.get(&lobby_id) else {
            self.reject(
                session,
                request_id,
                ErrorCode::NotInLobby,
                "the lobby is gone",
            );
            return None;
        };
        if &lobby.host != session {
            self.reject(
                session,
                request_id,
                ErrorCode::NotHost,
                "only the host can share a replay",
            );
            return None;
        }
        if !lobby.phase.allows_setup_changes() {
            self.reject(
                session,
                request_id,
                ErrorCode::WrongPhase,
                "not during a match",
            );
            return None;
        }
        Some(lobby_id)
    }

    pub(super) fn replay_validated(&mut self, validated: ValidatedReplay) {
        let ValidatedReplay {
            session,
            lobby: lobby_id,
            request_id,
            host_only_control,
            outcome,
        } = validated;
        if let Some(session_state) = self.sessions.get_mut(&session) {
            session_state.validating_replay = false;
        }
        let upload = match outcome {
            Ok(upload) => upload,
            Err(error) => {
                return self.reject(
                    &session,
                    request_id,
                    ErrorCode::InvalidReplay,
                    &error.to_string(),
                );
            }
        };
        // Things may have changed while the file was checked.
        match self.may_share_replay(&session, request_id) {
            Some(current) if current == lobby_id => {}
            // Refused already.
            None => return,
            Some(_) => {
                return self.reject(
                    &session,
                    request_id,
                    ErrorCode::NotInLobby,
                    "you moved to another lobby while the replay was checked",
                );
            }
        }
        let Some(lobby) = self.lobbies.get_mut(&lobby_id) else {
            return;
        };
        let ReplayUpload { push, end_tick } = upload;
        lobby.phase = LobbyPhase::Replay(SharedReplay {
            push: push.clone(),
            end_tick,
            host_only_control,
            speed: None,
            playback: None,
        });
        if let Some(lobby) = self.lobbies.get(&lobby_id) {
            self.send_to_members(lobby, &push);
        }
        self.ack(&session, request_id);
        self.after_lobby_change(&lobby_id);
    }

    pub(super) fn send_shared_replay(shared: &SharedReplay, outbound: &Outbound) {
        outbound.send_encoded(shared.push.clone());
        for control in [shared.speed, shared.playback].into_iter().flatten() {
            outbound.send(&codec::push(Message::ReplayControl(control)));
        }
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
        let LobbyPhase::Replay(shared) = &mut lobby.phase else {
            return self.reject(
                session,
                request_id,
                ErrorCode::WrongPhase,
                "no shared replay",
            );
        };
        if shared.host_only_control && &lobby.host != session {
            return self.reject(
                session,
                request_id,
                ErrorCode::NotHost,
                "only the host controls this replay",
            );
        }
        let action = proto::ReplayControlAction::try_from(control.action)
            .unwrap_or(proto::ReplayControlAction::Unspecified);
        match action {
            proto::ReplayControlAction::Unspecified => {
                return self.reject(
                    session,
                    request_id,
                    ErrorCode::InvalidRequest,
                    "unknown replay action",
                );
            }
            proto::ReplayControlAction::SetSpeed
                if !(MIN_REPLAY_SPEED_PERCENT..=MAX_REPLAY_SPEED_PERCENT)
                    .contains(&control.speed_percent) =>
            {
                return self.reject(
                    session,
                    request_id,
                    ErrorCode::InvalidRequest,
                    "speed must be 25..=400 percent",
                );
            }
            proto::ReplayControlAction::Seek if Tick::new(control.tick) > shared.end_tick => {
                return self.reject(
                    session,
                    request_id,
                    ErrorCode::InvalidRequest,
                    "seek past the end of the replay",
                );
            }
            proto::ReplayControlAction::SetSpeed => shared.speed = Some(control),
            proto::ReplayControlAction::Play
            | proto::ReplayControlAction::Pause
            | proto::ReplayControlAction::Seek => shared.playback = Some(control),
            proto::ReplayControlAction::Stop => lobby.phase = LobbyPhase::Open,
        }
        let encoded = codec::encode(&codec::push(Message::ReplayControl(control)));
        if let Some(lobby) = self.lobbies.get(&lobby_id) {
            self.send_to_members(lobby, &encoded);
        }
        self.ack(session, request_id);
        if action == proto::ReplayControlAction::Stop {
            self.after_lobby_change(&lobby_id);
        } else {
            self.touch(&lobby_id);
        }
    }
}

/// Runs on the blocking pool: decoding and checking a replay of up to 4 MiB, and encoding the
/// push every member receives, is too much work for the hub task.
fn validate_replay(share: proto::ShareReplay) -> Result<ReplayUpload, mg_registry::ReplayError> {
    let end_tick = load_replay_file(&share.replay_file)?.log.end();
    let push = codec::encode(&codec::push(Message::SharedReplay(proto::SharedReplay {
        replay_file: share.replay_file,
        host_only_control: share.host_only_control,
    })));
    Ok(ReplayUpload { push, end_tick })
}
