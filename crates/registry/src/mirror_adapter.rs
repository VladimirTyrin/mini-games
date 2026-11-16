use std::borrow::Cow;

use mg_core::{Clock, CompactLog, Match, PlayerSpec, Status, Tick, Viewer};
use mg_protocol::{WireGame, frame_from_wire, proto};

use crate::parse_settings;

use crate::apply_error::ApplyError;
use crate::create_error::CreateError;
use crate::dyn_mirror::DynMirror;
use crate::game_kind::GameKind;

#[derive(Debug)]
pub(crate) struct MirrorAdapter<G: WireGame> {
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

    fn apply_compact_log(
        &mut self,
        log: &CompactLog<proto::TickFrame>,
        stop: Tick,
    ) -> Result<(), ApplyError> {
        self.game_match.apply_compact_log_with(log, stop, |frame| {
            Ok::<_, ApplyError>(Cow::Owned(frame_from_wire::<G>(frame)?))
        })
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
    let settings = parse_settings::<G>(settings)?;
    let game_match = Match::<G>::new(&settings, players)?;
    Ok(Box::new(MirrorAdapter { kind, game_match }))
}
