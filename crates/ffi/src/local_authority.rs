use mg_core::{Slot, Status, Viewer};
use mg_protocol::{REPLAY_FORMAT_VERSION, outcome_to_wire, proto};
use mg_registry::{
    DynAuthority, DynMirror, create_authority_from_header, create_mirror_from_header,
};

use crate::authority_step::AuthorityStep;
use crate::ffi_error::FfiError;

/// A match run locally: the authority plus a mirror of it for rendering views (an authority has
/// none). Opaque to C.
#[derive(Debug)]
pub struct MgAuthority {
    header: proto::MatchHeader,
    seed: u64,
    authority: Box<dyn DynAuthority>,
    mirror: Box<dyn DynMirror>,
    /// A type-erased authority cannot be cloned, so a clone replays this history from the same
    /// seed. Cost grows with the match length, which is fine for search over short games.
    history: Vec<AuthorityStep>,
}

impl MgAuthority {
    pub(crate) fn new(header: proto::MatchHeader, seed: u64) -> Result<Self, FfiError> {
        let authority = create_authority_from_header(&header, seed)?;
        let mirror = create_mirror_from_header(&header)?;
        Ok(Self {
            header,
            seed,
            authority,
            mirror,
            history: Vec::new(),
        })
    }

    pub(crate) fn authority(&self) -> &dyn DynAuthority {
        self.authority.as_ref()
    }

    pub(crate) fn submit(
        &mut self,
        slot: Slot,
        command: proto::GameCommand,
    ) -> Result<(), FfiError> {
        self.authority.submit(slot, &command)?;
        self.history.push(AuthorityStep::Submit { slot, command });
        Ok(())
    }

    pub(crate) fn drop_player(&mut self, slot: Slot) {
        self.authority.drop_player(slot);
        self.history.push(AuthorityStep::DropPlayer(slot));
    }

    pub(crate) fn run_bots(&mut self) {
        self.authority.run_bots();
        self.history.push(AuthorityStep::RunBots);
    }

    pub(crate) fn seal_frame(&mut self) -> Result<proto::TickFrame, FfiError> {
        let frame = self.authority.seal_frame();
        self.history.push(AuthorityStep::SealFrame);
        self.mirror
            .apply_frame(&frame)
            .map_err(FfiError::OwnFrameRejected)?;
        Ok(frame)
    }

    pub(crate) fn fork(&self) -> Result<Self, FfiError> {
        let mut fork = Self::new(self.header.clone(), self.seed)?;
        for step in &self.history {
            match step {
                AuthorityStep::Submit { slot, command } => fork
                    .authority
                    .submit(*slot, command)
                    .map_err(FfiError::CloneDiverged)?,
                AuthorityStep::DropPlayer(slot) => fork.authority.drop_player(*slot),
                AuthorityStep::RunBots => fork.authority.run_bots(),
                AuthorityStep::SealFrame => {
                    let frame = fork.authority.seal_frame();
                    fork.mirror
                        .apply_frame(&frame)
                        .map_err(FfiError::OwnFrameRejected)?;
                }
            }
        }
        fork.history = self.history.clone();
        Ok(fork)
    }

    pub(crate) fn view(&self, viewer: Viewer) -> proto::GameView {
        self.mirror.view(viewer)
    }

    /// Present once the game is over.
    pub(crate) fn result(&self) -> Option<proto::MatchResult> {
        match self.authority.status() {
            Status::Finished(outcome) => Some(outcome_to_wire(&outcome)),
            Status::InProgress => None,
        }
    }

    pub(crate) fn replay_file(&self) -> proto::ReplayFile {
        proto::ReplayFile {
            format_version: REPLAY_FORMAT_VERSION,
            header: Some(self.header.clone()),
            frames: self.authority.compact_log().to_vec(),
            end_tick: self.authority.next_tick().get(),
            result: self.result(),
        }
    }
}
