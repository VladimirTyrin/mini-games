use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::Utf8Bytes;
use mg_core::{Awaiting, CancelToken, Clock, OutcomeKind, Slot, Status, Tick};
use mg_protocol::proto::server_message::Message;
use mg_protocol::{REPLAY_FORMAT_VERSION, proto, slot_from_wire};
use mg_registry::{DynAuthority, DynBotDecision, Rejection, SubmitError};
use prost::Message as _;
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinError;
use tokio::time::{Instant, MissedTickBehavior};

use super::cancel_on_drop::CancelOnDrop;
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
    /// The waits for disconnected seats: every one of them in a real-time game, only the one on
    /// turn in a turn-based game (see [`Self::update_waiting`]).
    budgets: DisconnectBudgets,
    /// Since when every human seat in play is disconnected.
    everyone_away_since: Option<Instant>,
    everyone_away_timeout: Duration,
    /// Everyone stayed away too long: the match ends without a result.
    abandoned: bool,
    state: RunState,
    /// Zero in turn-based games: a pause there holds back nobody's reflexes, so play goes on at
    /// once when the last player is back.
    resume_countdown: Duration,
    /// Catch-ups are the whole log; one encoding serves every request until the next frame.
    catch_up_cache: Option<CachedCatchUp>,
    hub: HubHandle,
    /// Caps the turn-based bot searches (seconds each) running at once. Real-time bots never
    /// take it: they decide every tick, and waiting behind a search would freeze their match.
    bot_cpu: Arc<Semaphore>,
    /// A turn-based bot is thinking on the blocking pool; its move comes back on `bot_moves`.
    bot_thinking: bool,
    bot_move_min: Duration,
    /// Each seat's secret as spectators last got it (games that show it to them).
    spectator_secrets: Vec<Option<proto::GameSeatSecret>>,
    /// Set when the actor runs.
    bot_moves: Option<BotMoveSender>,
    /// Cancelled when the actor is gone, so a bot still thinking for an ended match stops at once
    /// instead of holding a core (which real-time bots of other matches wait for).
    bot_cancel: CancelOnDrop,
    /// When people who have not sent their private command (a fleet) get one chosen for them.
    /// Runs on through pauses: a seat that is away when it ends gets a random fleet like anyone.
    secret_deadline: Option<Instant>,
}

type BotMoveSender = mpsc::UnboundedSender<Result<DynBotDecision, JoinError>>;

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
        let clock = setup.authority.clock();
        let secret_setup = Duration::from_millis(u64::from(setup.header.secret_setup_ms));
        Self {
            lobby: setup.lobby,
            match_id: setup.match_id,
            header: setup.header,
            clock,
            authority: Some(setup.authority),
            participants: setup.participants,
            dropped: BTreeSet::new(),
            // Waiting holds back everyone's reflexes in a real-time game, so it is budgeted; in a
            // turn-based one it holds back only the next move, and the others decide by vote.
            budgets: if clock.is_realtime() {
                DisconnectBudgets::new(
                    Some(setup.timing.disconnect_budget),
                    setup.timing.drop_vote_after,
                )
            } else {
                DisconnectBudgets::new(None, setup.timing.turn_drop_vote_after)
            },
            everyone_away_since: None,
            everyone_away_timeout: setup.timing.everyone_away_timeout,
            abandoned: false,
            state: RunState::Running,
            resume_countdown: if clock.is_realtime() {
                setup.timing.resume_countdown
            } else {
                Duration::ZERO
            },
            catch_up_cache: None,
            hub: setup.hub,
            bot_cpu: setup.bot_cpu,
            bot_thinking: false,
            bot_move_min: setup.timing.bot_move_min,
            spectator_secrets: Vec::new(),
            bot_moves: None,
            bot_cancel: CancelOnDrop::new(CancelToken::new()),
            secret_deadline: (!secret_setup.is_zero()).then(|| Instant::now() + secret_setup),
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
                Ok(MatchExit::Abandoned) => {
                    tracing::info!(%lobby, %match_id, "every player stayed away; match abandoned");
                }
                Err(error) => {
                    tracing::error!(%lobby, %match_id, %error, "match actor failed; aborting");
                }
            }
            hub.send(HubCommand::MatchAborted(MatchAborted { lobby, match_id }));
        });
    }

    async fn run(mut self, mut inbox: MatchInbox) -> MatchExit {
        let (bot_moves, mut decided_bot_moves) = mpsc::unbounded_channel();
        self.bot_moves = Some(bot_moves);
        self.broadcast(&codec::push(Message::MatchStarted(self.header.clone())));
        // Setup frame: initial randomness (first player, dealt board, spawned tiles).
        self.seal_and_broadcast();
        // Seats whose connection is already gone when the match starts are waited for as if they
        // had just lost it.
        self.update_waiting();
        if self.clock == Clock::TurnBased {
            self.drive_bots();
        }
        let tick_every = match self.clock {
            Clock::Realtime { tick_ms } => Duration::from_millis(u64::from(tick_ms.max(1))),
            Clock::TurnBased => Duration::from_secs(3600),
        };
        let mut ticker = tokio::time::interval(tick_every);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        ticker.tick().await;

        while !self.is_finished() && !self.abandoned {
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
                Some(decided) = decided_bot_moves.recv() => self.on_bot_move(decided),
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
        if self.abandoned {
            return MatchExit::Abandoned;
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
            MatchControl::Attach { session, outbound } => self.attach(&session, outbound).await,
            MatchControl::AddObserver { session, outbound } => {
                if self.participants.iter().all(|p| p.session != session) {
                    self.participants.push(Participant {
                        session: session.clone(),
                        slot: None,
                        outbound: None,
                    });
                }
                self.attach(&session, outbound).await;
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
            MatchRequest::SubmitSecret {
                session,
                request_id,
                command,
            } => self.submit_secret(&session, request_id, &command),
            MatchRequest::VoteDrop {
                session,
                request_id,
                slot,
            } => self.vote_drop(&session, request_id, slot).await,
            MatchRequest::CatchUp { session } => {
                if let Some(outbound) = self.outbound_of(&session).cloned() {
                    outbound.send_encoded(self.catch_up());
                }
                match self.slot_of(&session) {
                    Some(slot) => self.send_seat_secret(slot),
                    None => self.send_spectator_secrets(&session),
                }
            }
        }
    }

    async fn submit(&mut self, session: &SessionId, request_id: u32, command: &proto::GameCommand) {
        let Some(outbound) = self.outbound_of(session).cloned() else {
            return;
        };
        let refuse = |reason: proto::CommonRefusal| {
            outbound.send(&codec::command_rejected(
                request_id,
                Rejection::common(reason),
            ));
        };
        let Some(slot) = self.slot_of(session) else {
            return refuse(proto::CommonRefusal::NotPlaying);
        };
        if self.dropped.contains(&slot) {
            return refuse(proto::CommonRefusal::NotPlaying);
        }
        if self.state != RunState::Running {
            return refuse(proto::CommonRefusal::Paused);
        }
        let Some(authority) = self.authority.as_mut() else {
            return;
        };
        match authority.submit(slot, command) {
            Ok(()) => {
                outbound.send(&codec::reply(request_id, Message::Ack(proto::Ack {})));
                if self.clock == Clock::TurnBased {
                    self.seal_and_broadcast();
                    self.drive_bots();
                }
            }
            Err(SubmitError::Wire(error)) => {
                outbound.send(&codec::error(
                    request_id,
                    proto::ErrorCode::InvalidRequest,
                    error.to_string(),
                ));
            }
            Err(SubmitError::Rejected(rejection)) => {
                outbound.send(&codec::command_rejected(request_id, rejection));
            }
        }
    }

    fn submit_secret(
        &mut self,
        session: &SessionId,
        request_id: u32,
        command: &proto::GameSecretCommand,
    ) {
        let Some(outbound) = self.outbound_of(session).cloned() else {
            return;
        };
        let refuse = |reason: proto::CommonRefusal| {
            outbound.send(&codec::command_rejected(
                request_id,
                Rejection::common(reason),
            ));
        };
        let Some(slot) = self.slot_of(session) else {
            return refuse(proto::CommonRefusal::NotPlaying);
        };
        if self.dropped.contains(&slot) {
            return refuse(proto::CommonRefusal::NotPlaying);
        }
        let Some(authority) = self.authority.as_mut() else {
            return;
        };
        match authority.submit_secret(slot, command) {
            Ok(()) => {
                outbound.send(&codec::reply(request_id, Message::Ack(proto::Ack {})));
                self.send_seat_secret(slot);
                self.sync_spectator_secrets();
                // What the others may learn of it (the fleet is placed) goes out now in a
                // turn-based game; a real-time one sends it with the next tick, as sealing now
                // would step the game between two ticks.
                if self.clock == Clock::TurnBased {
                    self.seal_and_broadcast();
                    self.drive_bots();
                }
            }
            Err(SubmitError::Wire(error)) => {
                outbound.send(&codec::error(
                    request_id,
                    proto::ErrorCode::InvalidRequest,
                    error.to_string(),
                ));
            }
            Err(SubmitError::Rejected(rejection)) => {
                outbound.send(&codec::command_rejected(request_id, rejection));
            }
        }
    }

    /// To the seat's own connection only: what it may see of the game's secret.
    fn send_seat_secret(&self, slot: Slot) {
        let Some(secret) = self.authority.as_ref().and_then(|a| a.seat_secret(slot)) else {
            return;
        };
        let message = codec::push(Message::SeatSecret(proto::MatchSeatSecret {
            match_id: self.match_id.to_string(),
            secret: Some(secret),
            slot: None,
        }));
        for participant in self.participants.iter().filter(|p| p.slot == Some(slot)) {
            if let Some(outbound) = &participant.outbound {
                outbound.send(&message);
            }
        }
    }

    fn spectator_secret_message(
        &self,
        slot: usize,
        secret: &proto::GameSeatSecret,
    ) -> proto::ServerMessage {
        codec::push(Message::SeatSecret(proto::MatchSeatSecret {
            match_id: self.match_id.to_string(),
            secret: Some(secret.clone()),
            slot: u32::try_from(slot).ok(),
        }))
    }

    /// Every seat's secret to one spectator that joined or caught up (games that show them).
    fn send_spectator_secrets(&self, session: &SessionId) {
        let Some(outbound) = self.outbound_of(session) else {
            return;
        };
        for (slot, secret) in self.spectator_secrets.iter().enumerate() {
            if let Some(secret) = secret {
                outbound.send(&self.spectator_secret_message(slot, secret));
            }
        }
    }

    /// Sends spectators the seats' secrets that changed (a fleet placed, by a person, by a bot as
    /// a frame is sealed, or by the time limit). Nothing for games that keep them to the seat.
    fn sync_spectator_secrets(&mut self) {
        let Some(authority) = self.authority.as_ref() else {
            return;
        };
        let current: Vec<Option<proto::GameSeatSecret>> = (0..self.header.seats.len())
            .map(|slot| {
                u8::try_from(slot)
                    .ok()
                    .and_then(|slot| authority.spectator_seat_secret(Slot::new(slot)))
            })
            .collect();
        if current == self.spectator_secrets {
            return;
        }
        for (slot, secret) in current.iter().enumerate() {
            let Some(secret) = secret else { continue };
            if self.spectator_secrets.get(slot).and_then(Option::as_ref) == Some(secret) {
                continue;
            }
            let message = codec::encode(&self.spectator_secret_message(slot, secret));
            for participant in self.participants.iter().filter(|p| p.slot.is_none()) {
                if let Some(outbound) = &participant.outbound {
                    outbound.send_encoded(message.clone());
                }
            }
        }
        self.spectator_secrets = current;
    }

    /// The time for private commands is up: people who sent none get the game's choice.
    fn settle_late_secrets(&mut self) {
        let Some(authority) = self.authority.as_mut() else {
            return;
        };
        let late: Vec<Slot> = self
            .participants
            .iter()
            .filter_map(|p| p.slot)
            .filter(|slot| !self.dropped.contains(slot) && authority.awaits_secret(*slot))
            .collect();
        let settled: Vec<Slot> = late
            .into_iter()
            .filter(|slot| match authority.submit_default_secret(*slot) {
                Ok(()) => true,
                Err(error) => {
                    // The seat keeps owing its private command: the match cannot start.
                    tracing::error!(match_id = %self.match_id, %slot, %error, "no default private command");
                    false
                }
            })
            .collect();
        if settled.is_empty() {
            return;
        }
        for slot in settled {
            self.send_seat_secret(slot);
        }
        self.sync_spectator_secrets();
        // As in `submit_secret`: a real-time game sends it with the next tick.
        if self.clock == Clock::TurnBased {
            self.seal_and_broadcast();
            self.drive_bots();
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

    /// Turn-based: the bot on turn thinks on the blocking pool (under the CPU semaphore) with its
    /// own copy of the game, while the match keeps answering: a slow bot holds up nobody's
    /// leaving, chat or reconnect. Its move comes back to [`Self::on_bot_move`].
    fn drive_bots(&mut self) {
        if self.bot_thinking || self.state != RunState::Running {
            return;
        }
        let Some(bot_moves) = self.bot_moves.clone() else {
            return;
        };
        let Some(turn) = self.authority.as_mut().and_then(|a| a.begin_bot_turn()) else {
            return;
        };
        self.bot_thinking = true;
        let bot_cpu = self.bot_cpu.clone();
        let match_id = self.match_id.clone();
        let cancel = self.bot_cancel.token().clone();
        let earliest_move = tokio::time::Instant::now() + self.bot_move_min;
        tokio::spawn(async move {
            let permit = bot_cpu.acquire_owned().await;
            if cancel.is_cancelled() {
                return;
            }
            let thinking_cancel = cancel.clone();
            let decided = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                turn.think(&thinking_cancel)
            })
            .await;
            // A quick decision waits for the rest of the minimum, so people can follow it.
            tokio::time::sleep_until(earliest_move).await;
            if cancel.is_cancelled() {
                return;
            }
            if bot_moves.send(decided).is_err() {
                tracing::debug!(%match_id, "match over before its bot moved");
            }
        });
    }

    /// The match may have moved on meanwhile (a player left): the authority then drops the
    /// move and the bot thinks again. A panicking bot ends the match as aborted.
    fn on_bot_move(&mut self, decided: Result<DynBotDecision, JoinError>) {
        self.bot_thinking = false;
        match decided {
            Ok(decision) => {
                let Some(authority) = self.authority.as_mut() else {
                    return;
                };
                authority.finish_bot_turn(decision);
                if authority.has_pending_inputs() {
                    self.seal_and_broadcast();
                }
                self.drive_bots();
            }
            Err(error) => {
                tracing::error!(match_id = %self.match_id, %error, "bot task panicked");
                self.authority = None;
            }
        }
    }

    /// Real-time bots decide on the blocking pool, so many bots at once never stall the async
    /// workers, but without the CPU semaphore: a turn-based search holding it for seconds must
    /// not stop this match's clock. A panicking bot leaves the authority `None`, which ends the
    /// actor as aborted.
    async fn run_bots(&mut self) {
        let Some(mut authority) = self.authority.take() else {
            return;
        };
        let joined = tokio::task::spawn_blocking(move || {
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

    async fn attach(&mut self, session: &SessionId, outbound: Outbound) {
        let Some(participant) = self.participants.iter_mut().find(|p| &p.session == session) else {
            return;
        };
        participant.outbound = Some(outbound.clone());
        let slot = participant.slot;
        outbound.send_encoded(self.catch_up());
        match slot {
            Some(slot) => self.send_seat_secret(slot),
            None => self.send_spectator_secrets(session),
        }
        if slot.is_some() {
            self.update_waiting();
            // A turn-based game goes on at once when the last player is back (no countdown
            // whose end would do it): a bot on turn must move now, or nobody can.
            if self.clock == Clock::TurnBased {
                self.drive_bots();
            }
        }
    }

    fn detach(&mut self, session: &SessionId) {
        let Some(participant) = self.participants.iter_mut().find(|p| &p.session == session) else {
            return;
        };
        participant.outbound = None;
        if participant.slot.is_some() {
            self.update_waiting();
        }
    }

    /// Settles whom the match waits for, after anything that may change it (a seat lost or back,
    /// dropped, a move passing the turn): in a real-time game every seated human who is away, in
    /// a turn-based game only one the game itself waits on (on turn), so the others play on
    /// meanwhile. Starts and ends those waits, and pauses or resumes the match to match.
    fn update_waiting(&mut self) {
        if self.is_finished() {
            return;
        }
        let now = Instant::now();
        let in_play: Vec<&Participant> = self
            .participants
            .iter()
            .filter(|p| p.slot.is_some_and(|slot| self.is_in_play(slot)))
            .collect();
        let away: Vec<Slot> = in_play
            .iter()
            .filter(|p| p.outbound.is_none())
            .filter_map(|p| p.slot)
            .collect();
        let everyone_away = !away.is_empty() && away.len() == in_play.len();
        self.everyone_away_since = everyone_away.then(|| self.everyone_away_since.unwrap_or(now));
        let waited: BTreeSet<Slot> = away
            .into_iter()
            .filter(|&slot| self.waits_for(slot))
            .collect();
        let newly_waited: Vec<Slot> = waited
            .iter()
            .copied()
            .filter(|&slot| !self.budgets.is_disconnected(slot))
            .collect();
        let no_longer_waited: Vec<Slot> = self
            .budgets
            .disconnected()
            .filter(|slot| !waited.contains(slot))
            .collect();
        for &slot in &newly_waited {
            self.budgets.disconnect(slot, now);
        }
        for &slot in &no_longer_waited {
            self.budgets.reconnect(slot, now);
        }
        if let Some(&first) = newly_waited.first() {
            self.state = RunState::Paused;
            self.broadcast_pause(first, now);
        } else if !no_longer_waited.is_empty() {
            self.after_reconnect();
        }
    }

    /// Not dropped and not out of the game by its own rules.
    fn is_in_play(&self, slot: Slot) -> bool {
        !self.dropped.contains(&slot)
            && self
                .authority
                .as_ref()
                .is_some_and(|authority| !authority.has_left_play(slot))
    }

    /// Whether play stops while `slot` is away.
    fn waits_for(&self, slot: Slot) -> bool {
        match self.clock {
            Clock::Realtime { .. } => true,
            Clock::TurnBased => match self.authority.as_ref().map(|a| a.awaiting()) {
                Some(Awaiting::Turn(on_turn)) => on_turn == slot,
                Some(Awaiting::Simultaneous) => true,
                Some(Awaiting::Nobody) | None => false,
            },
        }
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
        if let Some(authority) = self.authority.as_mut() {
            authority.drop_player(slot);
        }
        // Sealing a real-time frame steps the game, so the drop waits for the next tick: sealing
        // it now would move everyone during a pause, or add a tick between two ticks. A drop that
        // ends the game goes out at once (a finished game does not step).
        if self.clock == Clock::TurnBased || self.is_finished() {
            self.seal_and_broadcast();
        }
        self.update_waiting();
        if self.clock == Clock::TurnBased {
            self.drive_bots();
        }
    }

    fn after_reconnect(&mut self) {
        if let Some(still_away) = self.budgets.disconnected().next() {
            self.broadcast_pause(still_away, Instant::now());
            return;
        }
        if self.state == RunState::Paused {
            self.state = if self.resume_countdown.is_zero() {
                RunState::Running
            } else {
                RunState::Countdown {
                    until: Instant::now() + self.resume_countdown,
                }
            };
            self.broadcast(&codec::push(Message::Resumed(proto::MatchResumed {
                match_id: self.match_id.to_string(),
                countdown_ms: millis(self.resume_countdown),
            })));
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        let run_state = match self.state {
            RunState::Countdown { until } => Some(until),
            RunState::Paused => self.budgets.next_exhaustion(Instant::now()),
            RunState::Running => None,
        };
        let everyone_away_until = self
            .everyone_away_since
            .map(|since| since + self.everyone_away_timeout);
        run_state
            .into_iter()
            .chain(self.secret_deadline)
            .chain(everyone_away_until)
            .min()
    }

    async fn on_deadline(&mut self) {
        let now = Instant::now();
        if self
            .everyone_away_since
            .is_some_and(|since| since + self.everyone_away_timeout <= now)
        {
            self.abandoned = true;
            return;
        }
        if self.secret_deadline.is_some_and(|deadline| deadline <= now) {
            self.secret_deadline = None;
            self.settle_late_secrets();
        }
        match self.state {
            RunState::Countdown { until } if until <= now => {
                self.state = RunState::Running;
                if self.clock == Clock::TurnBased {
                    self.drive_bots();
                }
            }
            RunState::Paused => {
                for slot in self.budgets.exhausted(now) {
                    self.drop_slot(slot).await;
                }
            }
            RunState::Countdown { .. } | RunState::Running => {}
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
        // Sealing settles the bots' private commands (their fleets).
        self.sync_spectator_secrets();
        // The turn may have passed to someone away, or from them.
        if self.clock == Clock::TurnBased {
            self.update_waiting();
        }
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
            budget_left_ms: self.budgets.budget_left(slot, now).map(millis),
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
