use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use tokio::sync::mpsc;
use tonic::Status;

use crate::{log, ClientId, ClientMessage, ServerMessage};

use crate::message_handler::{HandleResult, MessageHandler};
use crate::web_server::WebServerState;

pub async fn handle_websocket(socket: WebSocket, state: WebServerState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    let (tx, mut rx) = mpsc::channel::<Result<ServerMessage, Status>>(128);

    let send_task = tokio::spawn(async move {
        while let Some(result) = rx.recv().await {
            if let Ok(msg) = result {
                let mut buf = Vec::new();
                if msg.encode(&mut buf).is_ok()
                    && ws_sender.send(Message::Binary(buf.into())).await.is_err()
                {
                    break;
                }
            }
        }
    });

    let handler = MessageHandler::new(
        state.lobby_manager,
        state.broadcaster,
        state.session_manager,
    );

    let mut client_id_opt: Option<ClientId> = None;

    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(msg) => {
                let data = match msg {
                    Message::Binary(data) => data.to_vec(),
                    Message::Close(_) => break,
                    _ => continue,
                };

                let client_message = match ClientMessage::decode(data.as_slice()) {
                    Ok(m) => m,
                    Err(e) => {
                        log!("Failed to decode ClientMessage: {}", e);
                        continue;
                    }
                };

                match handler
                    .handle_message(client_message, &tx, &mut client_id_opt)
                    .await
                {
                    HandleResult::Continue => {}
                    HandleResult::Disconnect => break,
                }
            }
            Err(e) => {
                log!("WebSocket error: {}", e);
                break;
            }
        }
    }

    if let Some(client_id) = &client_id_opt {
        log!("WebSocket connection ended for client: {}", client_id);
        handler.handle_client_disconnected(client_id).await;
    }

    send_task.abort();
}
