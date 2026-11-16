use mg_protocol::proto;
use prost::Message;
use wasm_bindgen::JsError;

pub(crate) fn decode_header(bytes: &[u8]) -> Result<proto::MatchHeader, JsError> {
    proto::MatchHeader::decode(bytes).map_err(|e| JsError::new(&format!("bad match header: {e}")))
}
