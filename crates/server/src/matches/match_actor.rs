use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::Utf8Bytes;
use mg_core::{Clock, OutcomeKind, Slot, Status, Tick};
use mg_protocol::proto::server_message::Message;
use mg_protocol::{REPLAY_FORMAT_VERSION, proto, slot_from_wire};
use mg_registry::{DynAuthority, SubmitError};
use prost::Message as _;
use tokio::sync::Semaphore;
use tokio::time::{Instant, MissedTickBehavior};

use super::disconnect_budget::DisconnectBudgets;
use super::match_aborted::MatchAborted;
use super::match_control::MatchControl;
use super::match_exit::MatchExit;
use super::match_finished::MatchFinished;
use super::match_inbox::MatchInbox;
use super::match_request::MatchRequest;
use super::participant::Participant;
use crate::clock::sleep_until_opt;
use crate::codec;
use crate::hub::{HubCommand, HubHandle};
use crate::ids::{LobbyId, MatchId, SessionId};
use crate::outbound::Outbound;
use crate::timing::Timing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunState {
    Running,
    /// At least one seated human is disconnected.
    Paused,
    /// Everyone is back; frames resume at the deadline.
    Countdown {
        until: Instant,
    },
}

#[derive(Debug)]
struct CachedCatchUp {
    next_tick: Tick,
    encoded: Utf8Bytes,
}

/// Owns one match: the authority, its participants and the pause logic. Talks to the hub only
/// to report the end; everything else goes straight to participant connections.
pub(crate) struct MatchActor {
    lobby: LobbyId,
    match_id: MatchId,
    header: proto::MatchHeader,
    clock: Clock,
    /// `None` while bots compute on the blocking pool, and for good once a bot task panicked.
    authority: Option<Box<dyn DynAuthority>>,
    participants: Vec<Participant>,
    /// Seats out of play (left, voted out, budget spent). They never pause the match again.
    dropped: BTreeSet<Slot>,
    budgets: DisconnectBudgets,
    state: RunState,
    resume_countdown: Duration,
    /// Catch-ups are the whole log; one encoding serves every request until the next frame.
    catch_up_cache: Option<CachedCatchUp>,
    hub: HubHandle,
    bot_cpu: Arc<Semaphore>,
}

pub(crate) struct MatchSetup {
    pub lobby: LobbyId,
    pub match_id: MatchId,
    pub header: proto::MatchHeader,
    pub authority: Box<dyn DynAuthority>,
    pub participants: Vec<Participant>,
    pub hub: HubHandle,
    pub bot_cpu: Arc<Semaphore>,
    pub timing: Timing,
}

impl MatchActor {
    pub(crate) fn new(setup: MatchSetup) -> Self {
        Self {
            lobby: setup.lobby,
            match_id: setup.match_id,
            header: setup.header,
            clock: setup.authority.clock(),
            authority: Some(setup.authority),
            participants: setup.participants,
            dropped: BTreeSet::new(),
            budgets: DisconnectBudgets::new(
                setup.timing.disconnect_budget,
                setup.timing.drop_vote_after,
            ),
            state: RunState::Running,
            resume_countdown: setup.timing.resume_countdown,
            catch_up_cache: None,
            hub: setup.hub,
            bot_cpu: setup.bot_cpu,
        }
    }

    /// Runs the actor under a supervisor: whatever happens to it (even a panic), the hub learns
    /// that the match is over, so the lobby never stays in a match forever.
    pub(crate) fn spawn(self, inbox: MatchInbox) {
        let hub = self.hub.clone();
        let lobby = self.lobby.clone();
        let match_id = self.match_id.clone();
        let actor = tokio::spawn(self.run(inbox));
        tokio::spawn(async move {
            match actor.await {
                Ok(MatchExit::Finished | MatchExit::Stopped) => return,
                Ok(MatchExit::Aborted) => {
                    tracing::error!(%lobby, %match_id, "match lost its authority; aborting");
                }
                Err(error) => {
                    tracing::error!(%lobby, %match_id, %error, "match actor failed; aborting");
                }
            }
            hub.send(HubCommand::MatchAborted(MatchAborted { lobby, match_id }));
        });
    }

    async fn run(mut self, mut inbox: MatchInbox) -> MatchExit {
        self.broadcast(&codec::push(Message::MatchStarted(self.header.clone())));
        // Setup frame: initial randomness (first player, dealt board, spawned tiles).
        self.seal_and_broadcast();
        self.pause_for_absent_seats();
        if self.clock == Clock::TurnBased {
            self.drive_bots().await;
        }
        let tick_every = match self.clock {
            Clock::Realtime { tick_ms } => Duration::from_millis(u64::from(tick_ms.max(1))),
            Clock::TurnBased => Duration::from_secs(3600),
        };
        let mut ticker = tokio::time::interval(tick_every);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        ticker.tick().await;

        while !self.is_finished() {
            let deadline = self.next_deadline();
            let ticking = self.clock.is_realtime() && self.state == RunState::Running;
            tokio::select! {
                // Lifecycle first: an attach or leave must never wait behind queued requests.
                biased;
                control = inbox.control.recv() => match control {
                    Some(MatchControl::Stop) | None => return MatchExit::Stopped,
                    Some(control) => self.handle_control(control).await,
                },
                _ = ticker.tick(), if ticking => self.tick().await,
                () = sleep_until_opt(deadline), if deadline.is_some() => self.on_deadline().await,
                request = inbox.requests.recv() => match request {
                    Some(request) => self.handle_request(request).await,
                    None => return MatchExit::Stopped,
                },
            }
        }
        if self.authority.is_none() {
            return MatchExit::Aborted;
        }
        self.finish();
        MatchExit::Finished
    }

    fn is_finished(&self) -> bool {
        self.authority
            .as_ref()
            .is_none_or(|a| a.status().is_finished())
    }

    async fn handle_control(&mut self, control: MatchControl) {
        match control {
            MatchControl::Attach { session, outbound } => self.attach(&session, outbound),
            MatchControl::AddObserver { session, outbound } => {
                if self.participants.iter().all(|p| p.session != session) {
                    self.participants.push(Participant {
                        session: session.clone(),
                        slot: None,
                        outbound: None,
                    });
                }
                self.attach(&session, outbound);
            }
            MatchControl::Detach { session } => self.detach(&session),
            MatchControl::Leave { session } => self.leave(&session).await,
            MatchControl::Stop => {}
        }
    }

    async fn handle_request(&mut self, request: MatchRequest) {
        match request {
            MatchRequest::Submit {
                session,
                request_id,
                command,
            } => self.submit(&session, request_id, &command).await,
            MatchRequest::VoteDrop {
                session,
                request_id,
                slot,
            } => self.vote_drop(&session, request_id, slot).await,
            MatchRequest::CatchUp { session } => {
                if let Some(outbound) = self.outbound_of(&session).cloned() {
                    outbound.send_encoded(self.catch_up());
                }
            }
        }
    }

    async fn submit(&mut self, session: &SessionId, request_id: u32, command: &proto::GameCommand) {
        let Some(outbound) = self.outbound_of(session).cloned() else {
            return;
        };
        let reject = |code: proto::ErrorCode, text: &str| {
            outbound.send(&codec::error(request_id, code, text));
        };
        let Some(slot) = self.slot_of(session) else {
            return reject(proto::ErrorCode::CommandRejected, "observers cannot play");
        };
        if self.dropped.contains(&slot) {
            return reject(
                proto::ErrorCode::CommandRejected,
                "you were dropped from this match",
            );
        }
        if self.state != RunState::Running {
            return reject(proto::ErrorCode::CommandRejected, "the match is paused");
        }
        let Some(authority) = self.authority.as_mut() else {
            return;
        };
        match authority.submit(slot, command) {
            Ok(()) => {
                outbound.send(&codec::reply(request_id, Message::Ack(proto::Ack {})));
                if self.clock == Clock::TurnBased {
                    self.seal_and_broadcast();
                    self.drive_bots().await;
                }
            }
            Err(SubmitError::Wire(error)) => {
                reject(proto::ErrorCode::InvalidRequest, &error.to_string());
            }
            Err(SubmitError::Rejected(error)) => {
                reject(proto::ErrorCode::CommandRejected, &error.to_string());
            }
        }
    }

    /// Real-time tick: bots decide (on the blocking pool when the match has any), then the
    /// frame is sealed.
    async fn tick(&mut self) {
        if self
            .authority
            .as_ref()
            .is_some_and(|a| a.is_waiting_for_bot())
        {
            self.run_bots().await;
        }
        self.seal_and_broadcast();
    }

    /// Turn-based: lets bots move until a human is on turn.
    async fn drive_bots(&mut self) {
        loop {
            let waiting = self
                .authority
                .as_ref()
                .is_some_and(|a| a.is_waiting_for_bot() && !a.status().is_finished());
            if !waiting || self.state != RunState::Running {
                return;
            }
            self.run_bots().await;
            if self.authority.is_none() {
                return;
            }
            self.seal_and_broadcast();
        }
    }

    /// Bot decisions run on the blocking pool under the CPU semaphore, so a minimax search (or
    /// many real-time bots at once) never stalls the async workers. A panicking bot leaves the
    /// authority `None`, which ends the actor as aborted.
    async fn run_bots(&mut self) {
        let Some(mut authority) = self.authority.take() else {
            return;
        };
        let permit = self.bot_cpu.clone().acquire_owned().await;
        let joined = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            authority.run_bots();
            authority
        })
        .await;
        match joined {
            Ok(authority) => self.authority = Some(authority),
            Err(error) => {
                tracing::error!(match_id = %self.match_id, %error, "bot task panicked");
            }
        }
    }

    /// Seated humans whose connection is already gone when the match starts are treated as
    /// detached, so the match waits for them (with their budget) instead of running without
    /// them.
    fn pause_for_absent_seats(&mut self) {
        let now = Instant::now();
        let absent: Vec<Slot> = self
            .participants
            .iter()
            .filter(|p| p.outbound.is_none())
            .filter_map(|p| p.slot)
            .collect();
        for slot in &absent {
            self.budgets.disconnect(*slot, now);
        }
        if let Some(first) = absent.first() {
            self.state = RunState::Paused;
            self.broadcast_pause(*first, now);
        }
    }

    fn attach(&mut self, session: &SessionId, outbound: Outbound) {
        let Some(participant) = self.participants.iter_mut().find(|p| &p.session == session) else {
            return;
        };
        participant.outbound = Some(outbound.clone());
        let slot = participant.slot;
        outbound.send_encoded(self.catch_up());
        if let Some(slot) = slot
            && self.budgets.is_disconnected(slot)
        {
            self.budgets.reconnect(slot, Instant::now());
            self.after_reconnect();
        }
    }

    fn detach(&mut self, session: &SessionId) {
        let Some(participant) = self.participants.iter_mut().find(|p| &p.session == session) else {
            return;
        };
        participant.outbound = None;
        let Some(slot) = participant.slot else { return };
        if self.is_finished() || self.dropped.contains(&slot) {
            return;
        }
        let now = Instant::now();
        self.budgets.disconnect(slot, now);
        self.state = RunState::Paused;
        self.broadcast_pause(slot, now);
    }

    async fn leave(&mut self, session: &SessionId) {
        let Some(index) = self.participants.iter().position(|p| &p.session == session) else {
            return;
        };
        let participant = self.participants.remove(index);
        if let Some(slot) = participant.slot {
            self.drop_slot(slot).await;
        }
    }

    async fn vote_drop(&mut self, voter: &SessionId, request_id: u32, slot: u32) {
        let Some(outbound) = self.outbound_of(voter).cloned() else {
            return;
        };
        let reject = |code: proto::ErrorCode, text: &str| {
            outbound.send(&codec::error(request_id, code, text));
        };
        let target = match slot_from_wire(slot) {
            Ok(target) if self.budgets.is_disconnected(target) => target,
            _ => {
                return reject(
                    proto::ErrorCode::InvalidRequest,
                    "that seat is not disconnected",
                );
            }
        };
        if self
            .slot_of(voter)
            .is_none_or(|voter_slot| self.dropped.contains(&voter_slot))
        {
            return reject(proto::ErrorCode::InvalidRequest, "only players can vote");
        }
        if !self.budgets.drop_vote_in(target, Instant::now()).is_zero() {
            return reject(proto::ErrorCode::WrongPhase, "voting is not open yet");
        }
        outbound.send(&codec::reply(request_id, Message::Ack(proto::Ack {})));
        self.drop_slot(target).await;
    }

    async fn drop_slot(&mut self, slot: Slot) {
        if !self.dropped.insert(slot) {
            return;
        }
        let was_disconnected = self.budgets.is_disconnected(slot);
        self.budgets.forget(slot, Instant::now());
        if let Some(authority) = self.authority.as_mut() {
            authority.drop_player(slot);
        }
        // Sealing a real-time frame steps the game, so the drop waits for the next tick: sealing
        // it now would move everyone during a pause, or add a tick between two ticks. A drop that
        // ends the game goes out at once (a finished game does not step).
        if self.clock == Clock::TurnBased || self.is_finished() {
            self.seal_and_broadcast();
        }
        if was_disconnected {
            self.after_reconnect();
        }
        if self.clock == Clock::TurnBased {
            self.drive_bots().await;
        }
    }

    fn after_reconnect(&mut self) {
        if let Some(still_away) = self.budgets.disconnected().next() {
            self.broadcast_pause(still_away, Instant::now());
            return;
        }
        if self.state == RunState::Paused {
            self.state = RunState::Countdown {
                until: Instant::now() + self.resume_countdown,
            };
            self.broadcast(&codec::push(Message::Resumed(proto::MatchResumed {
                match_id: self.match_id.to_string(),
                countdown_ms: millis(self.resume_countdown),
            })));
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        match self.state {
            RunState::Countdown { until } => Some(until),
            RunState::Paused => self.budgets.next_exhaustion(Instant::now()),
            RunState::Running => None,
        }
    }

    async fn on_deadline(&mut self) {
        match self.state {
            RunState::Countdown { .. } => {
                self.state = RunState::Running;
                if self.clock == Clock::TurnBased {
                    self.drive_bots().await;
                }
            }
            RunState::Paused => {
                for slot in self.budgets.exhausted(Instant::now()) {
                    self.drop_slot(slot).await;
                }
            }
            RunState::Running => {}
        }
    }

    fn slot_of(&self, session: &SessionId) -> Option<Slot> {
        self.participants
            .iter()
            .find(|p| &p.session == session)
            .and_then(|p| p.slot)
    }

    fn outbound_of(&self, session: &SessionId) -> Option<&Outbound> {
        self.participants
            .iter()
            .find(|p| &p.session == session)
            .and_then(|p| p.outbound.as_ref())
    }

    fn seal_and_broadcast(&mut self) {
        let Some(authority) = self.authority.as_mut() else {
            return;
        };
        let frame = authority.seal_frame();
        self.broadcast(&codec::push(Message::Frame(frame)));
    }

    fn broadcast(&self, message: &proto::ServerMessage) {
        let encoded: Utf8Bytes = codec::encode(message);
        for outbound in self.participants.iter().filter_map(|p| p.outbound.as_ref()) {
            outbound.send_encoded(encoded.clone());
        }
    }

    /// `changed` is the seat this push is about; `pauses` always lists every seat still away,
    /// so a client replaces its list instead of guessing which seats came back.
    fn broadcast_pause(&self, changed: Slot, now: Instant) {
        self.broadcast(&codec::push(Message::Paused(proto::MatchPaused {
            match_id: self.match_id.to_string(),
            pause: Some(self.pause_info(changed, now)),
            pauses: self.current_pauses(now),
        })));
    }

    fn current_pauses(&self, now: Instant) -> Vec<proto::MatchPause> {
        self.budgets
            .disconnected()
            .map(|slot| self.pause_info(slot, now))
            .collect()
    }

    fn pause_info(&self, slot: Slot, now: Instant) -> proto::MatchPause {
        proto::MatchPause {
            slot: u32::from(slot.get()),
            budget_left_ms: millis(self.budgets.budget_left(slot, now)),
            drop_vote_in_ms: millis(self.budgets.drop_vote_in(slot, now)),
        }
    }

    /// Encoded `CatchUp` push. While running it depends only on the log, so it is cached until
    /// the next frame; paused or counting down it carries live timers and is built fresh.
    fn catch_up(&mut self) -> Utf8Bytes {
        let next_tick = self
            .authority
            .as_ref()
            .map_or(Tick::new(0), |a| a.next_tick());
        let cacheable = self.state == RunState::Running;
        if cacheable
            && let Some(cached) = &self.catch_up_cache
            && cached.next_tick == next_tick
        {
            return cached.encoded.clone();
        }
        let now = Instant::now();
        let pauses = self.current_pauses(now);
        let resume_countdown_ms = match self.state {
            RunState::Countdown { until } => millis(until.saturating_duration_since(now)),
            RunState::Running | RunState::Paused => 0,
        };
        let catch_up = proto::MatchCatchUp {
            header: Some(self.header.clone()),
            frames: self
                .authority
                .as_ref()
                .map(|a| a.compact_log().to_vec())
                .unwrap_or_default(),
            next_tick: next_tick.get(),
            pause: pauses.first().copied(),
            pauses,
            resume_countdown_ms,
        };
        let encoded = codec::encode(&codec::push(Message::CatchUp(catch_up)));
        if cacheable {
            self.catch_up_cache = Some(CachedCatchUp {
                next_tick,
                encoded: encoded.clone(),
            });
        }
        encoded
    }

    fn finish(self) {
        let Some(authority) = self.authority.as_ref() else {
            return;
        };
        let Status::Finished(outcome) = authority.status() else {
            return;
        };
        let result = proto::MatchResult {
            kind: outcome_kind(outcome.kind) as i32,
            winner_slots: outcome.winners.iter().map(|s| u32::from(s.get())).collect(),
            scores: outcome.scores,
        };
        let final_tick = authority.next_tick().get();
        self.broadcast(&codec::push(Message::Ended(proto::MatchEnded {
            match_id: self.match_id.to_string(),
            result: Some(result.clone()),
            final_tick,
        })));
        let replay = proto::ReplayFile {
            format_version: REPLAY_FORMAT_VERSION,
            header: Some(self.header.clone()),
            frames: authority.compact_log().to_vec(),
            end_tick: final_tick,
            result: Some(result.clone()),
        };
        let replay_data = proto::ReplayData {
            match_id: self.match_id.to_string(),
            replay_file: replay.encode_to_vec(),
            suggested_file_name: format!("match-{}.mgreplay", self.match_id),
        };
        self.hub.send(HubCommand::MatchFinished(MatchFinished {
            lobby: self.lobby,
            match_id: self.match_id,
            result,
            replay_data_json: codec::encode_json(&replay_data).into(),
        }));
    }
}

fn millis(duration: Duration) -> u32 {
    u32::try_from(duration.as_millis()).unwrap_or(u32::MAX)
}

fn outcome_kind(kind: OutcomeKind) -> proto::OutcomeKind {
    match kind {
        OutcomeKind::Winner => proto::OutcomeKind::Winner,
        OutcomeKind::Draw => proto::OutcomeKind::Draw,
        OutcomeKind::Victory => proto::OutcomeKind::Victory,
        OutcomeKind::Defeat => proto::OutcomeKind::Defeat,
    }
}

#[cfg(test)]
#[path = "match_actor_tests.rs"]
mod tests;
