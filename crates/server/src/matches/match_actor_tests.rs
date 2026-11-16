use mg_core::Awaiting;
use mg_registry::GameKind;
use tokio::sync::mpsc;

use super::*;
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
