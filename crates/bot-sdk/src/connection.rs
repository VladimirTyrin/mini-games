use futures_util::{SinkExt, StreamExt};
use mg_protocol::proto;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::sdk_error::SdkError;

/// One WebSocket carrying protobuf JSON text frames.
#[derive(Debug)]
pub(crate) struct Connection {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl Connection {
    pub(crate) async fn open(url: &str) -> Result<Self, SdkError> {
        let (socket, _handshake_response) = connect_async(url).await.map_err(SdkError::Connect)?;
        Ok(Self { socket })
    }

    pub(crate) async fn send(&mut self, message: &proto::ClientMessage) -> Result<(), SdkError> {
        let text = serde_json::to_string(message).map_err(SdkError::Encode)?;
        self.socket
            .send(WsMessage::Text(text.into()))
            .await
            .map_err(SdkError::WebSocket)
    }

    /// Cancel-safe: dropping the future loses no message.
    pub(crate) async fn receive(&mut self) -> Result<proto::ServerMessage, SdkError> {
        loop {
            match self.socket.next().await {
                None | Some(Ok(WsMessage::Close(_))) => return Err(SdkError::ConnectionClosed),
                Some(Err(error)) => return Err(SdkError::WebSocket(error)),
                Some(Ok(WsMessage::Text(text))) => {
                    return serde_json::from_str(text.as_str()).map_err(SdkError::Decode);
                }
                // tungstenite answers WebSocket pings itself.
                Some(Ok(_)) => {}
            }
        }
    }
}
