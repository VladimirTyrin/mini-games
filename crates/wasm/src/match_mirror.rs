use mg_core::{Clock, CompactLog, CoreRng, Slot, Status, Tick, Viewer};
use mg_protocol::proto;
use mg_registry::{
    DynMirror, LoadedReplay, SubmitError, create_mirror_from_header, load_replay_file,
};
use prost::Message;
use wasm_bindgen::prelude::*;

use crate::match_header::decode_header;
use crate::match_result::match_result;

/// Receiving side of lockstep in the page: applies frames (from the server, the local host or a
/// replay file) and renders views. Compact logs (catch-ups, replays) are expanded here, so the
/// page never synthesizes empty frames.
#[derive(Debug)]
#[wasm_bindgen]
pub struct MatchMirror {
    mirror: Box<dyn DynMirror>,
    /// Set by `fromReplayFile`, stepped through by `advanceTo`.
    replay_log: Option<CompactLog<proto::TickFrame>>,
}

#[wasm_bindgen]
impl MatchMirror {
    /// `header`: encoded `MatchHeader`.
    #[wasm_bindgen(constructor)]
    pub fn new(header: &[u8]) -> Result<MatchMirror, JsError> {
        let mirror = create_mirror_from_header(&decode_header(header)?)?;
        Ok(Self {
            mirror,
            replay_log: None,
        })
    }

    /// `replay`: encoded `ReplayFile`. Validates the whole file up front (format, header, seats,
    /// settings, frame log) and returns a mirror at tick 0 holding the log for `advanceTo`.
    /// Seeking backwards means building a new one.
    #[wasm_bindgen(js_name = fromReplayFile)]
    pub fn from_replay_file(replay: &[u8]) -> Result<MatchMirror, JsError> {
        let LoadedReplay { mirror, log } = load_replay_file(replay)?;
        Ok(Self {
            mirror,
            replay_log: Some(log),
        })
    }

    /// Milliseconds per frame for real-time games, 0 for turn-based ones. The game's own clock,
    /// so renderers interpolate the same way for local, online and replayed matches.
    #[wasm_bindgen(js_name = tickMs)]
    pub fn tick_ms(&self) -> u32 {
        match self.mirror.clock() {
            Clock::Realtime { tick_ms } => tick_ms,
            Clock::TurnBased => 0,
        }
    }

    /// Ticks covered by the replay log (its `end_tick`); 0 without one.
    #[wasm_bindgen(js_name = replayEndTick)]
    pub fn replay_end_tick(&self) -> u64 {
        self.replay_log.as_ref().map_or(0, |log| log.end().get())
    }

    /// Applies the replay log up to `stop_tick` (exclusive, capped at the end).
    #[wasm_bindgen(js_name = advanceTo)]
    pub fn advance_to(&mut self, stop_tick: u64) -> Result<(), JsError> {
        let log = self
            .replay_log
            .as_ref()
            .ok_or_else(|| JsError::new("this mirror has no replay log"))?;
        Ok(self.mirror.apply_compact_log(log, Tick::new(stop_tick))?)
    }

    /// `catch_up`: encoded `MatchCatchUp` (the page re-encodes the message it received, which is
    /// cheaper than shipping frames one by one). Its frames are validated as a whole, then the
    /// ticks from `nextTick` on are applied; the header is ignored. An error means a corrupt
    /// catch-up.
    #[wasm_bindgen(js_name = applyCatchUp)]
    pub fn apply_catch_up(&mut self, catch_up: &[u8]) -> Result<(), JsError> {
        let catch_up = proto::MatchCatchUp::decode(catch_up)?;
        Ok(self
            .mirror
            .catch_up(catch_up.frames, Tick::new(catch_up.next_tick))?)
    }

    /// `frame`: encoded `TickFrame`. An error means desync: rebuild from a full catch-up.
    #[wasm_bindgen(js_name = applyFrame)]
    pub fn apply_frame(&mut self, frame: &[u8]) -> Result<(), JsError> {
        let frame = proto::TickFrame::decode(frame)?;
        Ok(self.mirror.apply_frame(&frame)?)
    }

    /// Encoded `GameView`; `viewer_slot < 0` renders for a spectator.
    pub fn view(&self, viewer_slot: i32) -> Vec<u8> {
        let viewer = u8::try_from(viewer_slot)
            .map_or(Viewer::Spectator, |slot| Viewer::Player(Slot::new(slot)));
        self.mirror.view(viewer).encode_to_vec()
    }

    /// Why `slot` may not send `command` (encoded `GameCommand`) now, in words for the player;
    /// `undefined` when the game would accept it. The page asks this before offering an action
    /// instead of repeating the rule.
    #[wasm_bindgen(js_name = refusalOf)]
    pub fn refusal_of(&self, slot: u8, command: &[u8]) -> Result<Option<String>, JsError> {
        let command = proto::GameCommand::decode(command)?;
        match self.mirror.check_command(Slot::new(slot), &command) {
            Ok(()) => Ok(None),
            Err(SubmitError::Rejected(refusal)) => Ok(Some(refusal.explanation().to_owned())),
            Err(SubmitError::Wire(error)) => Err(error.into()),
        }
    }

    /// Encoded `MatchResult` when `command` from `slot` would end the match, empty when the match
    /// would go on; an error when the game refuses the command.
    /// Why the rules would refuse the private command (encoded `GameSecretCommand`) from `slot`,
    /// as far as the public state tells; `None` when they accept it.
    #[wasm_bindgen(js_name = secretRefusalOf)]
    pub fn secret_refusal_of(&self, slot: u8, command: &[u8]) -> Result<Option<String>, JsError> {
        let command = proto::GameSecretCommand::decode(command)?;
        Ok(
            match self.mirror.check_secret_command(Slot::new(slot), &command) {
                Ok(()) => None,
                Err(SubmitError::Rejected(error)) => Some(error.to_string()),
                Err(SubmitError::Wire(error)) => return Err(error.into()),
            },
        )
    }

    /// Encoded `GameSecretCommand` the game would choose for `slot` (a random fleet); empty when
    /// it has none. The seed comes from the page as two 32-bit halves.
    #[wasm_bindgen(js_name = defaultSecretCommand)]
    pub fn default_secret_command(&self, slot: u8, seed_high: u32, seed_low: u32) -> Vec<u8> {
        let mut rng = CoreRng::new((u64::from(seed_high) << 32) | u64::from(seed_low));
        self.mirror
            .default_secret_command(Slot::new(slot), &mut rng)
            .map(|command| command.encode_to_vec())
            .unwrap_or_default()
    }

    #[wasm_bindgen(js_name = resultAfter)]
    pub fn result_after(&self, slot: u8, command: &[u8]) -> Result<Vec<u8>, JsError> {
        let command = proto::GameCommand::decode(command)?;
        match self.mirror.status_after(Slot::new(slot), &command) {
            Ok(Status::Finished(outcome)) => Ok(match_result(outcome).encode_to_vec()),
            Ok(Status::InProgress) => Ok(Vec::new()),
            Err(SubmitError::Rejected(refusal)) => Err(JsError::new(refusal.explanation())),
            Err(SubmitError::Wire(error)) => Err(error.into()),
        }
    }

    /// Ticks are u64 in Rust; f64 keeps them exact far beyond any real match length.
    #[wasm_bindgen(js_name = nextTick)]
    pub fn next_tick(&self) -> f64 {
        self.mirror.next_tick().get() as f64
    }

    #[wasm_bindgen(js_name = isFinished)]
    pub fn is_finished(&self) -> bool {
        self.mirror.status().is_finished()
    }

    /// Encoded `MatchResult` once finished, otherwise empty.
    pub fn result(&self) -> Vec<u8> {
        match self.mirror.status() {
            Status::Finished(outcome) => match_result(outcome).encode_to_vec(),
            Status::InProgress => Vec::new(),
        }
    }

    #[wasm_bindgen(js_name = stateHash)]
    pub fn state_hash(&self) -> String {
        format!("{:016x}", self.mirror.state_hash())
    }
}
