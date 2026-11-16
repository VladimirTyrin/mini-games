use axum::extract::ws::Utf8Bytes;
use mg_protocol::proto;

pub(crate) fn decode(text: &str) -> Result<proto::ClientMessage, serde_json::Error> {
    serde_json::from_str(text)
}

pub(crate) fn encode(message: &proto::ServerMessage) -> Utf8Bytes {
    // Serializing generated protobuf types to a String cannot fail (no maps with non-string keys,
    // no custom serializers that error).
    serde_json::to_string(message).unwrap_or_default().into()
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
