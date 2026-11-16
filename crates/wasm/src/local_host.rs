use mg_core::{Clock, Slot, Status};
use mg_protocol::{REPLAY_FORMAT_VERSION, proto};
use mg_registry::{DynAuthority, create_authority_from_header};
use prost::Message;
use wasm_bindgen::prelude::*;

use crate::match_header::decode_header;
use crate::match_result::match_result;

/// Offline match authority (games without other humans). Runs in a Web Worker; the page drives
/// it with a timer for real-time games and feeds the sealed frames to its `MatchMirror`.
#[derive(Debug)]
#[wasm_bindgen]
pub struct LocalHost {
    authority: Box<dyn DynAuthority>,
    header: proto::MatchHeader,
}

#[wasm_bindgen]
impl LocalHost {
    /// `header`: encoded `MatchHeader` (settings + seats). The seed comes from the page's CSPRNG
    /// as two 32-bit halves (JS numbers cannot carry a full u64).
    #[wasm_bindgen(constructor)]
    pub fn new(header: &[u8], seed_high: u32, seed_low: u32) -> Result<LocalHost, JsError> {
        let header = decode_header(header)?;
        let seed = (u64::from(seed_high) << 32) | u64::from(seed_low);
        let authority = create_authority_from_header(&header, seed)?;
        Ok(Self { authority, header })
    }

    /// Milliseconds per frame for real-time games, 0 for turn-based ones.
    #[wasm_bindgen(js_name = tickMs)]
    pub fn tick_ms(&self) -> u32 {
        match self.authority.clock() {
            Clock::Realtime { tick_ms } => tick_ms,
            Clock::TurnBased => 0,
        }
    }

    /// `command`: encoded `GameCommand`.
    pub fn submit(&mut self, slot: u8, command: &[u8]) -> Result<(), JsError> {
        let command = proto::GameCommand::decode(command)?;
        Ok(self.authority.submit(Slot::new(slot), &command)?)
    }

    #[wasm_bindgen(js_name = dropPlayer)]
    pub fn drop_player(&mut self, slot: u8) {
        self.authority.drop_player(Slot::new(slot));
    }

    #[wasm_bindgen(js_name = runBots)]
    pub fn run_bots(&mut self) {
        self.authority.run_bots();
    }

    #[wasm_bindgen(js_name = isWaitingForBot)]
    pub fn is_waiting_for_bot(&self) -> bool {
        self.authority.is_waiting_for_bot()
    }

    #[wasm_bindgen(js_name = hasPendingInputs)]
    pub fn has_pending_inputs(&self) -> bool {
        self.authority.has_pending_inputs()
    }

    /// Encoded `TickFrame`.
    #[wasm_bindgen(js_name = sealFrame)]
    pub fn seal_frame(&mut self) -> Vec<u8> {
        self.authority.seal_frame().encode_to_vec()
    }

    #[wasm_bindgen(js_name = isFinished)]
    pub fn is_finished(&self) -> bool {
        self.authority.status().is_finished()
    }

    /// Encoded `ReplayFile` of the match so far (complete once finished).
    #[wasm_bindgen(js_name = replayFile)]
    pub fn replay_file(&self) -> Vec<u8> {
        let result = match self.authority.status() {
            Status::Finished(outcome) => Some(match_result(outcome)),
            Status::InProgress => None,
        };
        proto::ReplayFile {
            format_version: REPLAY_FORMAT_VERSION,
            header: Some(self.header.clone()),
            frames: self.authority.compact_log().to_vec(),
            end_tick: self.authority.next_tick().get(),
            result,
        }
        .encode_to_vec()
    }
}
