use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::Utf8Bytes;
use mg_core::{Clock, OutcomeKind, Slot, Status};
use mg_protocol::proto::server_message::Message;
use mg_protocol::{proto, slot_from_wire};
use mg_registry::{DynAuthority, SubmitError};
use prost::Message as _;
use tokio::sync::{Semaphore, mpsc};
use tokio::time::{Instant, MissedTickBehavior};

use super::disconnect_budget::DisconnectBudgets;
use super::match_command::MatchCommand;
use super::match_finished::MatchFinished;
use super::participant::Participant;
use crate::codec;
use crate::hub::{HubCommand, HubHandle};
use crate::ids::{LobbyId, MatchId, SessionId};
use crate::limits::Limits;
use crate::outbound::Outbound;

const REPLAY_FORMAT_VERSION: u32 = 2;

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

/// Owns one match: the authority, its participants and the pause logic. Talks to the hub only
/// to report the end; everything else goes straight to participant connections.
pub(crate) struct MatchActor {
    lobby: LobbyId,
    match_id: MatchId,
    header: proto::MatchHeader,
    /// `None` only while a bot computes on the blocking pool.
    authority: Option<Box<dyn DynAuthority>>,
    participants: Vec<Participant>,
    budgets: DisconnectBudgets,
    state: RunState,
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
}

impl MatchActor {
    pub(crate) fn new(setup: MatchSetup) -> Self {
        Self {
            lobby: setup.lobby,
            match_id: setup.match_id,
            header: setup.header,
            authority: Some(setup.authority),
            participants: setup.participants,
            budgets: DisconnectBudgets::new(Limits::DISCONNECT_BUDGET, Limits::DROP_VOTE_AFTER),
            state: RunState::Running,
            hub: setup.hub,
            bot_cpu: setup.bot_cpu,
        }
    }

    pub(crate) async fn run(mut self, mut commands: mpsc::Receiver<MatchCommand>) {
        self.broadcast(&codec::push(Message::MatchStarted(self.header.clone())));
        let Some(clock) = self.authority.as_ref().map(|a| a.clock()) else {
            return;
        };
        // Setup frame: initial randomness (first player, dealt board, spawned tiles).
        self.seal_and_broadcast();
        if clock == Clock::TurnBased {
            self.drive_bots().await;
        }
        let tick_every = match clock {
            Clock::Realtime { tick_ms } => Duration::from_millis(u64::from(tick_ms.max(1))),
            Clock::TurnBased => Duration::from_secs(3600),
        };
        let mut ticker = tokio::time::interval(tick_every);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        ticker.tick().await;

        while !self.is_finished() {
            let deadline = self.next_deadline();
            tokio::select! {
                command = commands.recv() => match command {
                    Some(MatchCommand::Stop) | None => return,
                    Some(command) => self.handle(command, clock).await,
                },
                _ = ticker.tick(), if clock.is_realtime() && self.state == RunState::Running => {
                    if let Some(authority) = self.authority.as_mut() {
                        authority.run_bots();
                    }
                    self.seal_and_broadcast();
                }
                () = sleep_until_opt(deadline), if deadline.is_some() => self.on_deadline(clock).await,
            }
        }
        self.finish();
    }

    fn is_finished(&self) -> bool {
        self.authority
            .as_ref()
            .is_none_or(|a| a.status().is_finished())
    }

    async fn handle(&mut self, command: MatchCommand, clock: Clock) {
        match command {
            MatchCommand::Submit {
                session,
                request_id,
                command,
            } => self.submit(&session, request_id, &command, clock).await,
            MatchCommand::Attach { session, outbound } => self.attach(&session, outbound, clock),
            MatchCommand::AddObserver { session, outbound } => {
                self.participants.push(Participant {
                    session: session.clone(),
                    slot: None,
                    outbound: None,
                });
                self.attach(&session, outbound, clock);
            }
            MatchCommand::Detach { session } => self.detach(&session),
            MatchCommand::Leave { session } => self.leave(&session, clock).await,
            MatchCommand::VoteDrop {
                session,
                request_id,
                slot,
            } => self.vote_drop(&session, request_id, slot, clock).await,
            MatchCommand::CatchUp { session } => {
                if let Some(outbound) = self.outbound_of(&session) {
                    outbound.send(&codec::push(Message::CatchUp(self.catch_up())));
                }
            }
            MatchCommand::Stop => {}
        }
    }

    async fn submit(
        &mut self,
        session: &SessionId,
        request_id: u32,
        command: &proto::GameCommand,
        clock: Clock,
    ) {
        let Some(outbound) = self.outbound_of(session).cloned() else {
            return;
        };
        let Some(slot) = self.slot_of(session) else {
            outbound.send(&codec::error(
                request_id,
                proto::ErrorCode::CommandRejected,
                "observers cannot play",
            ));
            return;
        };
        if self.state != RunState::Running {
            outbound.send(&codec::error(
                request_id,
                proto::ErrorCode::CommandRejected,
                "the match is paused",
            ));
            return;
        }
        let Some(authority) = self.authority.as_mut() else {
            return;
        };
        match authority.submit(slot, command) {
            Ok(()) => {
                outbound.send(&codec::reply(request_id, Message::Ack(proto::Ack {})));
                if clock == Clock::TurnBased {
                    self.seal_and_broadcast();
                    self.drive_bots().await;
                }
            }
            Err(SubmitError::Wire(error)) => {
                outbound.send(&codec::error(
                    request_id,
                    proto::ErrorCode::InvalidRequest,
                    error.to_string(),
                ));
            }
            Err(SubmitError::Rejected(error)) => {
                outbound.send(&codec::error(
                    request_id,
                    proto::ErrorCode::CommandRejected,
                    error.to_string(),
                ));
            }
        }
    }

    /// Turn-based: lets bots move until a human is on turn. Searches run on the blocking pool so
    /// a minimax move never stalls the async workers.
    async fn drive_bots(&mut self) {
        loop {
            let waiting = self
                .authority
                .as_ref()
                .is_some_and(|a| a.is_waiting_for_bot() && !a.status().is_finished());
            if !waiting || self.state != RunState::Running {
                return;
            }
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
                    tracing::error!(match_id = %self.match_id, %error, "bot task panicked; aborting match");
                    return;
                }
            }
            self.seal_and_broadcast();
        }
    }

    fn attach(&mut self, session: &SessionId, outbound: Outbound, clock: Clock) {
        let Some(participant) = self.participants.iter_mut().find(|p| &p.session == session) else {
            return;
        };
        participant.outbound = Some(outbound.clone());
        let slot = participant.slot;
        outbound.send(&codec::push(Message::CatchUp(self.catch_up())));
        if let Some(slot) = slot
            && self.budgets.is_disconnected(slot)
        {
            self.budgets.reconnect(slot, std::time::Instant::now());
            self.after_reconnect(clock);
        }
    }

    fn detach(&mut self, session: &SessionId) {
        let Some(participant) = self.participants.iter_mut().find(|p| &p.session == session) else {
            return;
        };
        participant.outbound = None;
        let Some(slot) = participant.slot else { return };
        if self.is_finished() || self.is_dropped(slot) {
            return;
        }
        let now = std::time::Instant::now();
        self.budgets.disconnect(slot, now);
        self.state = RunState::Paused;
        self.broadcast_pause(slot, now);
    }

    async fn leave(&mut self, session: &SessionId, clock: Clock) {
        let Some(index) = self.participants.iter().position(|p| &p.session == session) else {
            return;
        };
        let participant = self.participants.remove(index);
        if let Some(slot) = participant.slot {
            self.drop_slot(slot, clock).await;
        }
    }

    async fn vote_drop(&mut self, voter: &SessionId, request_id: u32, slot: u32, clock: Clock) {
        let Some(outbound) = self.outbound_of(voter).cloned() else {
            return;
        };
        let target = match slot_from_wire(slot) {
            Ok(target) if self.budgets.is_disconnected(target) => target,
            _ => {
                outbound.send(&codec::error(
                    request_id,
                    proto::ErrorCode::InvalidRequest,
                    "that seat is not disconnected",
                ));
                return;
            }
        };
        if self.slot_of(voter).is_none() {
            outbound.send(&codec::error(
                request_id,
                proto::ErrorCode::InvalidRequest,
                "only players can vote",
            ));
            return;
        }
        if !self
            .budgets
            .drop_vote_in(target, std::time::Instant::now())
            .is_zero()
        {
            outbound.send(&codec::error(
                request_id,
                proto::ErrorCode::WrongPhase,
                "voting is not open yet",
            ));
            return;
        }
        outbound.send(&codec::reply(request_id, Message::Ack(proto::Ack {})));
        self.drop_slot(target, clock).await;
    }

    async fn drop_slot(&mut self, slot: Slot, clock: Clock) {
        let was_disconnected = self.budgets.is_disconnected(slot);
        self.budgets.forget(slot);
        if let Some(authority) = self.authority.as_mut() {
            authority.drop_player(slot);
        }
        self.seal_and_broadcast();
        if was_disconnected {
            self.after_reconnect(clock);
        }
        if clock == Clock::TurnBased {
            self.drive_bots().await;
        }
    }

    fn after_reconnect(&mut self, _clock: Clock) {
        if self.budgets.any_disconnected() {
            let now = std::time::Instant::now();
            for slot in self.budgets.disconnected().collect::<Vec<_>>() {
                self.broadcast_pause(slot, now);
            }
            return;
        }
        if self.state == RunState::Paused {
            self.state = RunState::Countdown {
                until: Instant::now() + Limits::RESUME_COUNTDOWN,
            };
            let countdown_ms =
                u32::try_from(Limits::RESUME_COUNTDOWN.as_millis()).unwrap_or(u32::MAX);
            self.broadcast(&codec::push(Message::Resumed(proto::MatchResumed {
                match_id: self.match_id.to_string(),
                countdown_ms,
            })));
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        match self.state {
            RunState::Countdown { until } => Some(until),
            RunState::Paused => {
                let now = std::time::Instant::now();
                self.budgets
                    .next_exhaustion(now)
                    .map(|at| Instant::now() + at.saturating_duration_since(now))
            }
            RunState::Running => None,
        }
    }

    async fn on_deadline(&mut self, clock: Clock) {
        match self.state {
            RunState::Countdown { .. } => {
                self.state = RunState::Running;
                if clock == Clock::TurnBased {
                    self.drive_bots().await;
                }
            }
            RunState::Paused => {
                for slot in self.budgets.exhausted(std::time::Instant::now()) {
                    self.drop_slot(slot, clock).await;
                }
            }
            RunState::Running => {}
        }
    }

    fn is_dropped(&self, slot: Slot) -> bool {
        self.participants.iter().all(|p| p.slot != Some(slot))
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

    fn broadcast_pause(&self, slot: Slot, now: std::time::Instant) {
        let pause = self.pause_info(slot, now);
        self.broadcast(&codec::push(Message::Paused(proto::MatchPaused {
            match_id: self.match_id.to_string(),
            pause: Some(pause),
        })));
    }

    fn pause_info(&self, slot: Slot, now: std::time::Instant) -> proto::MatchPause {
        let millis = |d: Duration| u32::try_from(d.as_millis()).unwrap_or(u32::MAX);
        proto::MatchPause {
            slot: u32::from(slot.get()),
            budget_left_ms: millis(self.budgets.budget_left(slot, now)),
            drop_vote_in_ms: millis(self.budgets.drop_vote_in(slot, now)),
        }
    }

    fn catch_up(&self) -> proto::MatchCatchUp {
        let now = std::time::Instant::now();
        let (frames, next_tick) = self
            .authority
            .as_ref()
            .map_or((Vec::new(), 0), |a| (a.compact_log(), a.next_tick().get()));
        proto::MatchCatchUp {
            header: Some(self.header.clone()),
            frames,
            next_tick,
            pause: self
                .budgets
                .disconnected()
                .next()
                .map(|slot| self.pause_info(slot, now)),
        }
    }

    fn finish(self) {
        let Some(authority) = self.authority.as_ref() else {
            return;
        };
        let result = match authority.status() {
            Status::Finished(outcome) => proto::MatchResult {
                kind: outcome_kind(outcome.kind) as i32,
                winner_slots: outcome.winners.iter().map(|s| u32::from(s.get())).collect(),
                scores: outcome.scores,
            },
            Status::InProgress => return,
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
            frames: authority.compact_log(),
            end_tick: final_tick,
            result: Some(result.clone()),
        };
        self.hub.send(HubCommand::MatchFinished(MatchFinished {
            lobby: self.lobby,
            match_id: self.match_id,
            result,
            replay: Arc::from(replay.encode_to_vec()),
        }));
    }
}

fn outcome_kind(kind: OutcomeKind) -> proto::OutcomeKind {
    match kind {
        OutcomeKind::Winner => proto::OutcomeKind::Winner,
        OutcomeKind::Draw => proto::OutcomeKind::Draw,
        OutcomeKind::Victory => proto::OutcomeKind::Victory,
        OutcomeKind::Defeat => proto::OutcomeKind::Defeat,
    }
}

async fn sleep_until_opt(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}
