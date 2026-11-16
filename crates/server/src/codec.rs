use axum::extract::ws::Utf8Bytes;
use mg_protocol::proto;

pub(crate) fn decode(text: &str) -> Result<proto::ClientMessage, serde_json::Error> {
    serde_json::from_str(text)
}

pub(crate) fn encode(message: &proto::ServerMessage) -> Utf8Bytes {
    encode_json(message).into()
}

/// Any generated message in its canonical JSON form.
pub(crate) fn encode_json(message: &impl serde::Serialize) -> String {
    // Serializing generated protobuf types to a String cannot fail (no maps with non-string keys,
    // no custom serializers that error).
    serde_json::to_string(message).unwrap_or_default()
}

/// A reply whose payload was encoded once up front (a replay is megabytes of base64): only the
/// envelope is written per request. `field` is the payload's JSON name in `ServerMessage`.
pub(crate) fn reply_with_encoded_payload(
    request_id: u32,
    field: &str,
    payload_json: &str,
) -> Utf8Bytes {
    let envelope = if request_id == 0 {
        format!("{{\"{field}\":{payload_json}}}")
    } else {
        format!("{{\"requestId\":{request_id},\"{field}\":{payload_json}}}")
    };
    envelope.into()
}

pub(crate) fn push(message: proto::server_message::Message) -> proto::ServerMessage {
    proto::ServerMessage {
        request_id: 0,
        message: Some(message),
    }
}

pub(crate) fn reply(
    request_id: u32,
    message: proto::server_message::Message,
) -> proto::ServerMessage {
    proto::ServerMessage {
        request_id,
        message: Some(message),
    }
}

pub(crate) fn error(
    request_id: u32,
    code: proto::ErrorCode,
    text: impl Into<String>,
) -> proto::ServerMessage {
    reply(
        request_id,
        proto::server_message::Message::Error(proto::ErrorResponse {
            code: code as i32,
            message: text.into(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spliced_reply_decodes_like_an_encoded_one() {
        let data = proto::ReplayData {
            match_id: "m".into(),
            replay_file: vec![1, 2, 3],
            suggested_file_name: "m.mgreplay".into(),
        };
        let expected = reply(7, proto::server_message::Message::ReplayData(data.clone()));
        let spliced = reply_with_encoded_payload(7, "replayData", &encode_json(&data));
        let decoded: proto::ServerMessage = serde_json::from_str(spliced.as_str()).unwrap();
        assert_eq!(decoded, expected);
    }
}
