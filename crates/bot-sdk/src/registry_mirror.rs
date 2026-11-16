use mg_core::{Clock, CompactLog, Slot, Status, Tick, Viewer};
use mg_protocol::proto;
use mg_registry::{
    ApplyError, CreateError, DynMirror, GameKind, SubmitError, create_mirror_from_header,
};

/// The registry's mirror for any game, for strategies that only need views.
#[derive(Debug)]
pub struct RegistryMirror(Box<dyn DynMirror>);

impl RegistryMirror {
    pub fn new(header: &proto::MatchHeader) -> Result<Self, CreateError> {
        create_mirror_from_header(header).map(Self)
    }
}

impl DynMirror for RegistryMirror {
    fn kind(&self) -> GameKind {
        self.0.kind()
    }

    fn clock(&self) -> Clock {
        self.0.clock()
    }

    fn apply_frame(&mut self, frame: &proto::TickFrame) -> Result<(), ApplyError> {
        self.0.apply_frame(frame)
    }

    fn apply_compact_log(
        &mut self,
        log: &CompactLog<proto::TickFrame>,
        stop: Tick,
    ) -> Result<(), ApplyError> {
        self.0.apply_compact_log(log, stop)
    }

    fn view(&self, viewer: Viewer) -> proto::GameView {
        self.0.view(viewer)
    }

    fn check_command(&self, slot: Slot, command: &proto::GameCommand) -> Result<(), SubmitError> {
        self.0.check_command(slot, command)
    }

    fn status_after(
        &self,
        slot: Slot,
        command: &proto::GameCommand,
    ) -> Result<Status, SubmitError> {
        self.0.status_after(slot, command)
    }

    fn status(&self) -> Status {
        self.0.status()
    }

    fn next_tick(&self) -> Tick {
        self.0.next_tick()
    }

    fn state_hash(&self) -> u64 {
        self.0.state_hash()
    }
}
