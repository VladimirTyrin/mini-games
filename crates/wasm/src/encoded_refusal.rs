use mg_registry::SubmitError;
use prost::Message;
use wasm_bindgen::prelude::*;

/// A refused command as an encoded `Refusal` for the page to word; `None` when accepted. Input
/// the page cannot have meant (another game's command) stays an error.
pub(crate) fn encoded_refusal(answer: Result<(), SubmitError>) -> Result<Option<Vec<u8>>, JsError> {
    match answer {
        Ok(()) => Ok(None),
        Err(SubmitError::Rejected(rejection)) => Ok(Some(rejection.refusal.encode_to_vec())),
        Err(SubmitError::Wire(error)) => Err(error.into()),
    }
}
