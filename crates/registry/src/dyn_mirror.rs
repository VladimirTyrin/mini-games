use mg_core::{Clock, Match, PlayerSpec, Status, Tick, Viewer};
use mg_protocol::{WireGame, frame_from_wire, proto};

use crate::game_kind::GameKind;
use crate::registry_error::{ApplyError, CreateError};

/// [`Match`] with the game type erased; the receiving side of lockstep.
pub trait DynMirror: Send + std::fmt::Debug {
    fn kind(&self) -> GameKind;
    fn clock(&self) -> Clock;
    fn apply_frame(&mut self, frame: &proto::TickFrame) -> Result<(), ApplyError>;
    fn view(&self, viewer: Viewer) -> proto::GameView;
    fn status(&self) -> Status;
    fn next_tick(&self) -> Tick;
    fn state_hash(&self) -> u64;
}

#[derive(Debug)]
struct MirrorAdapter<G: WireGame> {
    kind: GameKind,
    game_match: Match<G>,
}

impl<G: WireGame> DynMirror for MirrorAdapter<G> {
    fn kind(&self) -> GameKind {
        self.kind
    }

    fn clock(&self) -> Clock {
        self.game_match.game().clock()
    }

    fn apply_frame(&mut self, frame: &proto::TickFrame) -> Result<(), ApplyError> {
        let frame = frame_from_wire::<G>(frame)?;
        Ok(self.game_match.apply_frame(&frame)?)
    }

    fn view(&self, viewer: Viewer) -> proto::GameView {
        G::view_to_wire(&self.game_match.game().view(viewer))
    }

    fn status(&self) -> Status {
        self.game_match.game().status()
    }

    fn next_tick(&self) -> Tick {
        self.game_match.next_tick()
    }

    fn state_hash(&self) -> u64 {
        self.game_match.game().state_hash()
    }
}

pub(crate) fn mirror<G: WireGame>(
    kind: GameKind,
    settings: &proto::GameSettings,
    players: &[PlayerSpec],
) -> Result<Box<dyn DynMirror>, CreateError> {
    let settings = G::settings_from_wire(settings)?;
    let game_match = Match::<G>::new(&settings, players)?;
    Ok(Box::new(MirrorAdapter { kind, game_match }))
}
