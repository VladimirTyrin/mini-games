use std::sync::Mutex;

use mg_core::{Awaiting, Outcome};
use mg_registry::{DynBotTurn, GameKind};
use tokio::sync::mpsc;

use super::*;
use crate::ids::ConnectionId;
use crate::limits::Limits;
use crate::matches::MatchHandle;

#[derive(Debug, Clone, Copy)]
enum Failure {
    /// The actor itself panics (sealing the setup frame).
    InActor,
    /// A bot search panics on the blocking pool.
    InBotTask,
}

/// A turn-based authority that fails in a chosen way.
#[derive(Debug)]
struct FailingAuthority {
    failure: Failure,
}

impl DynAuthority for FailingAuthority {
    fn kind(&self) -> GameKind {
        GameKind::ALL[0]
    }
    fn clock(&self) -> Clock {
        Clock::TurnBased
    }
    fn submit(&mut self, _slot: Slot, _command: &proto::GameCommand) -> Result<(), SubmitError> {
        Ok(())
    }
    fn drop_player(&mut self, _slot: Slot) {}
    fn run_bots(&mut self) {
        panic!("bot search failed");
    }
    fn is_waiting_for_bot(&self) -> bool {
        matches!(self.failure, Failure::InBotTask)
    }
    fn begin_bot_turn(&mut self) -> Option<Box<dyn DynBotTurn>> {
        matches!(self.failure, Failure::InBotTask).then(|| Box::new(PanickingTurn) as _)
    }
    fn finish_bot_turn(&mut self, _decision: DynBotDecision) {}
    fn has_pending_inputs(&self) -> bool {
        false
    }
    fn seal_frame(&mut self) -> proto::TickFrame {
        if let Failure::InActor = self.failure {
            panic!("sealing failed");
        }
        proto::TickFrame::default()
    }
    fn awaiting(&self) -> Awaiting {
        Awaiting::Nobody
    }
    fn status(&self) -> Status {
        Status::InProgress
    }
    fn next_tick(&self) -> Tick {
        Tick::new(0)
    }
    fn compact_log(&self) -> &[proto::TickFrame] {
        &[]
    }
}

struct PanickingTurn;

impl DynBotTurn for PanickingTurn {
    fn think(self: Box<Self>, _cancel: &CancelToken) -> DynBotDecision {
        panic!("bot search failed");
    }
}

async fn run_until_hub_hears(failure: Failure) -> HubCommand {
    let (hub_sender, mut hub_commands) = mpsc::unbounded_channel();
    let lobby = LobbyId::random();
    let match_id = MatchId::random();
    let (_handle, inbox) = MatchHandle::channel(match_id.clone(), Limits::MATCH_REQUEST_QUEUE);
    MatchActor::new(MatchSetup {
        lobby,
        match_id,
        header: proto::MatchHeader::default(),
        authority: Box::new(FailingAuthority { failure }),
        participants: Vec::new(),
        hub: HubHandle::new(hub_sender),
        bot_cpu: Arc::new(Semaphore::new(1)),
        timing: Timing::default(),
    })
    .spawn(inbox);
    hub_commands.recv().await.expect("the hub must hear back")
}

#[tokio::test]
async fn a_panicking_actor_is_reported_as_aborted() {
    let command = run_until_hub_hears(Failure::InActor).await;
    assert!(
        matches!(command, HubCommand::MatchAborted(_)),
        "{command:?}"
    );
}

#[tokio::test]
async fn a_panicking_bot_task_is_reported_as_aborted() {
    let command = run_until_hub_hears(Failure::InBotTask).await;
    assert!(
        matches!(command, HubCommand::MatchAborted(_)),
        "{command:?}"
    );
}

type Gate = Arc<Mutex<Option<std::sync::mpsc::Receiver<()>>>>;

/// A turn-based match where a bot is always on turn and thinks until `gate` is released,
/// announcing on `thinking` that it started; the one human seat (slot 0) leaving ends it.
#[derive(Debug)]
struct SlowBotAuthority {
    gate: Gate,
    thinking: mpsc::UnboundedSender<CancelToken>,
    finished: bool,
}

struct GatedTurn {
    gate: Gate,
    thinking: mpsc::UnboundedSender<CancelToken>,
}

impl DynBotTurn for GatedTurn {
    fn think(self: Box<Self>, cancel: &CancelToken) -> DynBotDecision {
        self.thinking.send(cancel.clone()).unwrap();
        let gate = self.gate.lock().unwrap().take();
        if let Some(gate) = gate {
            // Released (or the test is over) when the sender is dropped.
            gate.recv().ok();
        }
        DynBotDecision::new(())
    }
}

impl DynAuthority for SlowBotAuthority {
    fn kind(&self) -> GameKind {
        GameKind::ALL[0]
    }
    fn clock(&self) -> Clock {
        Clock::TurnBased
    }
    fn submit(&mut self, _slot: Slot, _command: &proto::GameCommand) -> Result<(), SubmitError> {
        Ok(())
    }
    fn drop_player(&mut self, _slot: Slot) {
        self.finished = true;
    }
    fn run_bots(&mut self) {}
    fn is_waiting_for_bot(&self) -> bool {
        !self.finished
    }
    fn begin_bot_turn(&mut self) -> Option<Box<dyn DynBotTurn>> {
        (!self.finished).then(|| {
            Box::new(GatedTurn {
                gate: self.gate.clone(),
                thinking: self.thinking.clone(),
            }) as _
        })
    }
    fn finish_bot_turn(&mut self, _decision: DynBotDecision) {}
    fn has_pending_inputs(&self) -> bool {
        false
    }
    fn seal_frame(&mut self) -> proto::TickFrame {
        proto::TickFrame::default()
    }
    fn awaiting(&self) -> Awaiting {
        Awaiting::Turn(Slot::new(1))
    }
    fn status(&self) -> Status {
        if self.finished {
            Status::Finished(Outcome {
                kind: mg_core::OutcomeKind::Winner,
                winners: vec![Slot::new(1)],
                scores: Vec::new(),
            })
        } else {
            Status::InProgress
        }
    }
    fn next_tick(&self) -> Tick {
        Tick::new(0)
    }
    fn compact_log(&self) -> &[proto::TickFrame] {
        &[]
    }
}

#[tokio::test]
async fn a_player_leaves_while_a_bot_is_still_thinking_and_the_bot_is_stopped() {
    let (release, gate) = std::sync::mpsc::channel::<()>();
    let gate = Arc::new(Mutex::new(Some(gate)));
    let (thinking, mut started_thinking) = mpsc::unbounded_channel();
    let (hub_sender, mut hub_commands) = mpsc::unbounded_channel();
    let match_id = MatchId::random();
    let (handle, inbox) = MatchHandle::channel(match_id.clone(), Limits::MATCH_REQUEST_QUEUE);
    let session = SessionId::random();
    let (outbound, _received) = Outbound::channel(ConnectionId::next());
    MatchActor::new(MatchSetup {
        lobby: LobbyId::random(),
        match_id,
        header: proto::MatchHeader::default(),
        authority: Box::new(SlowBotAuthority {
            gate: gate.clone(),
            thinking,
            finished: false,
        }),
        participants: vec![Participant {
            session: session.clone(),
            slot: Some(Slot::new(0)),
            outbound: Some(outbound),
        }],
        hub: HubHandle::new(hub_sender),
        bot_cpu: Arc::new(Semaphore::new(1)),
        timing: Timing::default(),
    })
    .spawn(inbox);

    let bot_cancel = started_thinking.recv().await.expect("the bot thinks");

    handle.control(MatchControl::Leave { session });

    let command = hub_commands.recv().await.expect("the hub must hear back");
    assert!(
        matches!(command, HubCommand::MatchFinished(_)),
        "{command:?}"
    );
    // The single-threaded test runtime ran the actor to its end (and drop) before this line.
    assert!(
        bot_cancel.is_cancelled(),
        "the ended match's bot must stop thinking"
    );
    drop(release);
}

/// A real-time match with a bot that reports every decision on `decided`.
#[derive(Debug)]
struct RealtimeBotAuthority {
    decided: mpsc::UnboundedSender<()>,
}

impl DynAuthority for RealtimeBotAuthority {
    fn kind(&self) -> GameKind {
        GameKind::ALL[0]
    }
    fn clock(&self) -> Clock {
        Clock::Realtime { tick_ms: 1 }
    }
    fn submit(&mut self, _slot: Slot, _command: &proto::GameCommand) -> Result<(), SubmitError> {
        Ok(())
    }
    fn drop_player(&mut self, _slot: Slot) {}
    fn run_bots(&mut self) {
        self.decided.send(()).ok();
    }
    fn is_waiting_for_bot(&self) -> bool {
        true
    }
    fn begin_bot_turn(&mut self) -> Option<Box<dyn DynBotTurn>> {
        None
    }
    fn finish_bot_turn(&mut self, _decision: DynBotDecision) {}
    fn has_pending_inputs(&self) -> bool {
        false
    }
    fn seal_frame(&mut self) -> proto::TickFrame {
        proto::TickFrame::default()
    }
    fn awaiting(&self) -> Awaiting {
        Awaiting::Nobody
    }
    fn status(&self) -> Status {
        Status::InProgress
    }
    fn next_tick(&self) -> Tick {
        Tick::new(0)
    }
    fn compact_log(&self) -> &[proto::TickFrame] {
        &[]
    }
}

#[tokio::test]
async fn real_time_bots_decide_while_turn_based_searches_hold_every_cpu_permit() {
    let bot_cpu = Arc::new(Semaphore::new(1));
    // A Master search elsewhere on the host holds the only permit for the whole test.
    let _searching = bot_cpu.clone().acquire_owned().await.unwrap();
    let (decided, mut decisions) = mpsc::unbounded_channel();
    let (hub_sender, _hub_commands) = mpsc::unbounded_channel();
    let match_id = MatchId::random();
    let (handle, inbox) = MatchHandle::channel(match_id.clone(), Limits::MATCH_REQUEST_QUEUE);
    MatchActor::new(MatchSetup {
        lobby: LobbyId::random(),
        match_id,
        header: proto::MatchHeader::default(),
        authority: Box::new(RealtimeBotAuthority { decided }),
        participants: Vec::new(),
        hub: HubHandle::new(hub_sender),
        bot_cpu,
        timing: Timing::default(),
    })
    .spawn(inbox);

    for _ in 0..3 {
        decisions.recv().await.expect("the real-time bot decides");
    }
    handle.control(MatchControl::Stop);
}

/// A turn-based match of three humans in which slot 2 has left play (gave up) and watches on.
#[derive(Debug)]
struct OneSeatGaveUpAuthority;

const GAVE_UP: Slot = Slot::new(2);

impl DynAuthority for OneSeatGaveUpAuthority {
    fn kind(&self) -> GameKind {
        GameKind::ALL[0]
    }
    fn clock(&self) -> Clock {
        Clock::TurnBased
    }
    fn submit(&mut self, _slot: Slot, _command: &proto::GameCommand) -> Result<(), SubmitError> {
        Ok(())
    }
    fn drop_player(&mut self, _slot: Slot) {}
    fn has_left_play(&self, slot: Slot) -> bool {
        slot == GAVE_UP
    }
    fn run_bots(&mut self) {}
    fn is_waiting_for_bot(&self) -> bool {
        false
    }
    fn begin_bot_turn(&mut self) -> Option<Box<dyn DynBotTurn>> {
        None
    }
    fn finish_bot_turn(&mut self, _decision: DynBotDecision) {}
    fn has_pending_inputs(&self) -> bool {
        false
    }
    fn seal_frame(&mut self) -> proto::TickFrame {
        proto::TickFrame::default()
    }
    /// Seat 1, so losing it pauses the match; the seat that gave up is never on turn.
    fn awaiting(&self) -> Awaiting {
        Awaiting::Turn(Slot::new(1))
    }
    fn status(&self) -> Status {
        Status::InProgress
    }
    fn next_tick(&self) -> Tick {
        Tick::new(0)
    }
    fn compact_log(&self) -> &[proto::TickFrame] {
        &[]
    }
}

#[tokio::test]
async fn a_player_who_left_play_pauses_nobody_when_their_connection_drops() {
    let (hub_sender, _hub_commands) = mpsc::unbounded_channel();
    let match_id = MatchId::random();
    let (handle, inbox) = MatchHandle::channel(match_id.clone(), Limits::MATCH_REQUEST_QUEUE);
    let sessions: Vec<SessionId> = (0..3).map(|_| SessionId::random()).collect();
    let mut receivers = Vec::new();
    let participants = sessions
        .iter()
        .enumerate()
        .map(|(index, session)| {
            let (outbound, receiver) = Outbound::channel(ConnectionId::next());
            receivers.push(receiver);
            Participant {
                session: session.clone(),
                slot: Some(Slot::new(u8::try_from(index).unwrap())),
                outbound: Some(outbound),
            }
        })
        .collect();
    MatchActor::new(MatchSetup {
        lobby: LobbyId::random(),
        match_id,
        header: proto::MatchHeader::default(),
        authority: Box::new(OneSeatGaveUpAuthority),
        participants,
        hub: HubHandle::new(hub_sender),
        bot_cpu: Arc::new(Semaphore::new(1)),
        timing: Timing::default(),
    })
    .spawn(inbox);

    // Handled in this order: had the first one paused the match, its pause would come first.
    handle.control(MatchControl::Detach {
        session: sessions[usize::from(GAVE_UP.get())].clone(),
    });
    handle.control(MatchControl::Detach {
        session: sessions[1].clone(),
    });

    let first_seat = &mut receivers[0];
    let pause = loop {
        let text = first_seat.next().await.expect("the match goes on talking");
        let message: proto::ServerMessage = serde_json::from_str(text.as_str()).unwrap();
        if let Some(Message::Paused(paused)) = message.message {
            break paused.pause.expect("a pause names its seat");
        }
    };
    assert_eq!(
        pause.slot, 1,
        "only the seat still playing pauses the match"
    );
    handle.control(MatchControl::Stop);
}

/// A turn-based match whose bot decides at once; the actor reports when it plays the move.
#[derive(Debug)]
struct QuickBotAuthority {
    played: mpsc::UnboundedSender<tokio::time::Instant>,
    moved: bool,
}

struct QuickTurn;

impl DynBotTurn for QuickTurn {
    fn think(self: Box<Self>, _cancel: &CancelToken) -> DynBotDecision {
        DynBotDecision::new(())
    }
}

impl DynAuthority for QuickBotAuthority {
    fn kind(&self) -> GameKind {
        GameKind::ALL[0]
    }
    fn clock(&self) -> Clock {
        Clock::TurnBased
    }
    fn submit(&mut self, _slot: Slot, _command: &proto::GameCommand) -> Result<(), SubmitError> {
        Ok(())
    }
    fn drop_player(&mut self, _slot: Slot) {}
    fn run_bots(&mut self) {}
    fn is_waiting_for_bot(&self) -> bool {
        !self.moved
    }
    fn begin_bot_turn(&mut self) -> Option<Box<dyn DynBotTurn>> {
        (!self.moved).then(|| Box::new(QuickTurn) as _)
    }
    fn finish_bot_turn(&mut self, _decision: DynBotDecision) {
        self.moved = true;
        self.played.send(tokio::time::Instant::now()).ok();
    }
    fn has_pending_inputs(&self) -> bool {
        false
    }
    fn seal_frame(&mut self) -> proto::TickFrame {
        proto::TickFrame::default()
    }
    fn awaiting(&self) -> Awaiting {
        Awaiting::Turn(Slot::new(0))
    }
    fn status(&self) -> Status {
        Status::InProgress
    }
    fn next_tick(&self) -> Tick {
        Tick::new(0)
    }
    fn compact_log(&self) -> &[proto::TickFrame] {
        &[]
    }
}

#[tokio::test]
async fn a_bot_that_decides_at_once_still_takes_the_minimum_time_to_move() {
    let bot_move_min = Duration::from_millis(60);
    let (played, mut moves) = mpsc::unbounded_channel();
    let (hub_sender, _hub_commands) = mpsc::unbounded_channel();
    let match_id = MatchId::random();
    let (handle, inbox) = MatchHandle::channel(match_id.clone(), Limits::MATCH_REQUEST_QUEUE);
    let started = tokio::time::Instant::now();
    MatchActor::new(MatchSetup {
        lobby: LobbyId::random(),
        match_id,
        header: proto::MatchHeader::default(),
        authority: Box::new(QuickBotAuthority {
            played,
            moved: false,
        }),
        participants: Vec::new(),
        hub: HubHandle::new(hub_sender),
        bot_cpu: Arc::new(Semaphore::new(1)),
        timing: Timing {
            bot_move_min,
            ..Timing::default()
        },
    })
    .spawn(inbox);

    let played_at = moves.recv().await.expect("the bot moves");
    // A lower bound only: the real clock can be late, never early.
    assert!(
        played_at - started >= bot_move_min,
        "{:?}",
        played_at - started
    );
    handle.control(MatchControl::Stop);
}
