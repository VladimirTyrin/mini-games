use std::borrow::Cow;

use mg_core::{Clock, CompactLog, CoreRng, Match, Slot, Status, Tick, Viewer};
use mg_protocol::{WireError, WireGame, frame_from_wire, proto};
use mg_registry::{
    ApplyError, CreateError, DynMirror, GameKind, SubmitError, parse_settings, players_from_header,
};

/// A mirror that keeps the concrete game type, so strategies can read the full game state (the
/// built-in bots need it). Behaves exactly like the registry's mirror.
#[derive(Debug)]
pub struct TypedMirror<G: WireGame> {
    kind: GameKind,
    game_match: Match<G>,
}

impl<G: WireGame> TypedMirror<G> {
    pub fn new(header: &proto::MatchHeader) -> Result<Self, CreateError> {
        let settings = header
            .settings
            .as_ref()
            .ok_or(WireError::Missing("header.settings"))?;
        let kind = GameKind::of_settings(settings)?;
        if kind.info().id != G::INFO.id {
            return Err(WireError::WrongGame.into());
        }
        let game_match = Match::new(
            &parse_settings::<G>(settings)?,
            &players_from_header(header)?,
        )?;
        Ok(Self { kind, game_match })
    }

    pub fn game(&self) -> &G {
        self.game_match.game()
    }
}

impl<G: WireGame> DynMirror for TypedMirror<G> {
    fn kind(&self) -> GameKind {
        self.kind
    }

    fn clock(&self) -> Clock {
        self.game().clock()
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
        G::view_to_wire(&self.game().view(viewer))
    }

    fn check_command(&self, slot: Slot, command: &proto::GameCommand) -> Result<(), SubmitError> {
        let command = G::command_from_wire(command)?;
        Ok(self.game_match.check_command(slot, &command)?)
    }

    fn status_after(
        &self,
        slot: Slot,
        command: &proto::GameCommand,
    ) -> Result<Status, SubmitError> {
        let command = G::command_from_wire(command)?;
        Ok(self.game_match.preview_command(slot, &command)?.status())
    }

    fn check_secret_command(
        &self,
        slot: Slot,
        command: &proto::GameSecretCommand,
    ) -> Result<(), SubmitError> {
        let command = G::secret_command_from_wire(command)?;
        Ok(self.game_match.check_secret_command(slot, &command)?)
    }

    fn default_secret_command(
        &self,
        slot: Slot,
        rng: &mut CoreRng,
    ) -> Option<proto::GameSecretCommand> {
        self.game_match
            .default_secret_command(slot, rng)
            .map(|command| G::secret_command_to_wire(&command))
    }

    fn status(&self) -> Status {
        self.game().status()
    }

    fn next_tick(&self) -> Tick {
        self.game_match.next_tick()
    }

    fn state_hash(&self) -> u64 {
        self.game().state_hash()
    }
}
