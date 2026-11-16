use std::sync::Arc;

use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures_util::{Sink, SinkExt, StreamExt};
use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as ClientPayload;
use mg_protocol::proto::server_message::Message;
use tokio::sync::{oneshot, watch};
use tokio::time::Instant;

use crate::auth::{AuthError, DisplayName, SessionKind};
use crate::authenticated::Authenticated;
use crate::clock::sleep_until_opt;
use crate::codec;
use crate::context::{PROTOCOL_VERSION, ServerContext};
use crate::hub::{HubCommand, SignIn};
use crate::ids::{ConnectionId, ResumeToken, SessionId};
use crate::limits::Limits;
use crate::outbound::{Outbound, OutboundReceiver};
use crate::token_bucket::TokenBucket;

use proto::ErrorCode;

struct Rejection {
    code: ErrorCode,
    message: String,
}

impl Rejection {
    fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<AuthError> for Rejection {
    fn from(error: AuthError) -> Self {
        let code = match error {
            AuthError::InvalidDisplayName(_) => ErrorCode::InvalidRequest,
            AuthError::GuestsDisabled => ErrorCode::GuestsDisabled,
            AuthError::GoogleKeysRetryLater => ErrorCode::RateLimited,
            _ => ErrorCode::SignInFailed,
        };
        Self::new(code, error.to_string())
    }
}

/// Result of a successful Hello.
struct SignedIn {
    who: Authenticated,
    /// Long-lived token for `SessionCredentials`; empty for guests.
    session_token: String,
}

struct Connection {
    id: ConnectionId,
    context: Arc<ServerContext>,
    outbound: Outbound,
    session: Option<SessionId>,
    /// Until the first successful sign-in: an anonymous connection must say Hello by then.
    hello_deadline: Option<Instant>,
    messages: TokenBucket,
    chat: TokenBucket,
    /// Frames above `MAX_MESSAGE_BYTES` (shared replays), checked before decoding them.
    large_messages: TokenBucket,
    /// Set after a fatal error (e.g. unsupported protocol): stop reading.
    finished: bool,
}

#[derive(Debug, thiserror::Error)]
enum WriteError {
    #[error("socket write failed")]
    Socket,
    #[error("socket write timed out")]
    Timeout,
}

pub(crate) async fn run(context: Arc<ServerContext>, socket: WebSocket) {
    let id = ConnectionId::next();
    let (outbound, receiver) = Outbound::channel(id);
    let (sink, mut stream) = socket.split();
    let mut writer = tokio::spawn(write_loop(sink, receiver));
    let now = Instant::now();
    let timing = context.timing;
    let mut connection = Connection {
        id,
        context,
        outbound,
        session: None,
        hello_deadline: Some(now + timing.hello_timeout),
        messages: TokenBucket::new(Limits::MESSAGE_BURST, Limits::MESSAGE_REFILL, now),
        chat: TokenBucket::new(Limits::CHAT_BURST, Limits::CHAT_REFILL, now),
        large_messages: TokenBucket::new(1, Limits::LARGE_MESSAGE_COOLDOWN, now),
        finished: false,
    };
    tracing::debug!(connection = %id, "connected");
    let mut shutting_down = connection.context.shutting_down.clone();

    while !connection.finished {
        let hello_deadline = connection.hello_deadline;
        let anonymous = connection.session.is_none();
        tokio::select! {
            next = tokio::time::timeout(timing.idle_timeout, stream.next()) => match next {
                Ok(Some(Ok(WsMessage::Text(text)))) => connection.handle_text(text.as_str()).await,
                Ok(Some(Ok(WsMessage::Binary(_)))) => connection.reject(0, ErrorCode::InvalidRequest, "send JSON text frames"),
                Ok(Some(Ok(WsMessage::Ping(_) | WsMessage::Pong(_)))) => {}
                Ok(Some(Ok(WsMessage::Close(_)) | Err(_)) | None) => break,
                Err(_idle) => {
                    tracing::debug!(connection = %id, "idle timeout");
                    break;
                }
            },
            () = sleep_until_opt(hello_deadline), if hello_deadline.is_some() => {
                tracing::debug!(connection = %id, "no Hello in time");
                break;
            }
            () = server_stopping(&mut shutting_down), if anonymous => break,
            _ = &mut writer => break,
        }
    }

    if let Some(session) = connection.session.take() {
        connection.context.hub.send(HubCommand::Disconnected {
            session,
            connection: id,
        });
    }
    connection.outbound.close();
    if !writer.is_finished()
        && tokio::time::timeout(Limits::SHUTDOWN_GRACE, &mut writer)
            .await
            .is_err()
    {
        writer.abort();
    }
    tracing::debug!(connection = %id, "disconnected");
}

/// Resolves once the server is stopping (or its context is gone).
async fn server_stopping(shutting_down: &mut watch::Receiver<bool>) {
    let stopping = shutting_down.wait_for(|stopping| *stopping).await.is_ok();
    tracing::trace!(stopping, "server shutdown observed");
}

async fn send_with_timeout<S>(sink: &mut S, message: WsMessage) -> Result<(), WriteError>
where
    S: Sink<WsMessage> + Unpin,
{
    match tokio::time::timeout(Limits::SEND_TIMEOUT, sink.send(message)).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(_)) => Err(WriteError::Socket),
        Err(_elapsed) => Err(WriteError::Timeout),
    }
}

async fn write_loop<S>(mut sink: S, mut outbound: OutboundReceiver)
where
    S: Sink<WsMessage> + Unpin,
{
    let written = write_queued(&mut sink, &mut outbound).await;
    if let Err(error) = written {
        tracing::debug!(%error, "connection writer stopped");
        return;
    }
    if let Err(error) = send_with_timeout(&mut sink, WsMessage::Close(None)).await {
        tracing::trace!(%error, "close frame not sent");
    }
}

async fn write_queued<S>(sink: &mut S, outbound: &mut OutboundReceiver) -> Result<(), WriteError>
where
    S: Sink<WsMessage> + Unpin,
{
    loop {
        tokio::select! {
            // A close request wins over queued messages so an overloaded client is cut off at
            // once; a normal close still flushes what was queued before it.
            biased;
            () = outbound.close_requested() => {
                if !outbound.is_overloaded() {
                    while let Some(text) = outbound.try_next() {
                        send_with_timeout(sink, WsMessage::Text(text)).await?;
                    }
                }
                return Ok(());
            }
            message = outbound.next() => match message {
                Some(_) if outbound.is_overloaded() => return Ok(()),
                Some(text) => send_with_timeout(sink, WsMessage::Text(text)).await?,
                None => return Ok(()),
            },
        }
    }
}

impl Connection {
    fn reject(&self, request_id: u32, code: ErrorCode, message: &str) {
        self.outbound.send(&codec::error(request_id, code, message));
    }

    async fn handle_text(&mut self, text: &str) {
        let now = Instant::now();
        if !self.messages.try_take(now) {
            return self.reject(0, ErrorCode::RateLimited, "slow down");
        }
        let large = text.len() > Limits::MAX_MESSAGE_BYTES;
        if large && self.session.is_none() {
            // Only a signed-in host may upload a replay; anything else this big is abuse and
            // is not worth decoding.
            self.reject(0, ErrorCode::InvalidRequest, "message too large");
            self.finished = true;
            return;
        }
        if large && !self.large_messages.try_take(now) {
            return self.reject(0, ErrorCode::RateLimited, "too many large messages");
        }
        let message = match codec::decode(text) {
            Ok(message) => message,
            Err(error) => {
                return self.reject(
                    0,
                    ErrorCode::InvalidRequest,
                    &format!("malformed message: {error}"),
                );
            }
        };
        let request_id = message.request_id;
        let Some(payload) = message.message else {
            return self.reject(request_id, ErrorCode::InvalidRequest, "empty message");
        };
        if large && !matches!(payload, ClientPayload::ShareReplay(_)) {
            return self.reject(request_id, ErrorCode::InvalidRequest, "message too large");
        }
        match payload {
            ClientPayload::Ping(ping) => {
                let pong = proto::Pong {
                    client_time_ms: ping.client_time_ms,
                };
                self.outbound
                    .send(&codec::reply(request_id, Message::Pong(pong)));
            }
            ClientPayload::ServerInfo(_) => {
                self.outbound.send(&codec::reply(
                    request_id,
                    Message::ServerInfo(self.context.server_info()),
                ));
            }
            ClientPayload::Hello(hello) => self.hello(request_id, hello).await,
            ClientPayload::SignOut(_) => {
                if let Some(session) = self.session.take() {
                    self.context.hub.send(HubCommand::SignOut {
                        session,
                        connection: self.id,
                    });
                }
                self.outbound
                    .send(&codec::reply(request_id, Message::Ack(proto::Ack {})));
            }
            payload => {
                let Some(session) = self.session.clone() else {
                    return self.reject(request_id, ErrorCode::NotSignedIn, "sign in first");
                };
                if matches!(payload, ClientPayload::SendChat(_)) && !self.chat.try_take(now) {
                    return self.reject(
                        request_id,
                        ErrorCode::RateLimited,
                        "too many chat messages",
                    );
                }
                self.context.hub.send(HubCommand::Client {
                    session,
                    connection: self.id,
                    request_id,
                    message: payload,
                });
            }
        }
    }

    async fn hello(&mut self, request_id: u32, hello: proto::Hello) {
        if self.session.is_some() {
            return self.reject(request_id, ErrorCode::InvalidRequest, "already signed in");
        }
        if hello.protocol_version != PROTOCOL_VERSION {
            self.reject(
                request_id,
                ErrorCode::ProtocolUnsupported,
                &format!("this server speaks protocol {PROTOCOL_VERSION}; please update the app"),
            );
            self.finished = true;
            return;
        }
        let SignedIn { who, session_token } = match self.authenticate(hello.credentials).await {
            Ok(signed_in) => signed_in,
            Err(rejection) => return self.reject(request_id, rejection.code, &rejection.message),
        };
        let (reply, response) = oneshot::channel();
        self.context.hub.send(HubCommand::SignIn(SignIn {
            connection: self.id,
            outbound: self.outbound.clone(),
            who,
            session_token,
            resume: ResumeToken::parse(&hello.resume_token),
            server_info: self.context.server_info(),
            request_id,
            reply,
        }));
        self.session = response.await.ok().flatten();
        if self.session.is_some() {
            self.hello_deadline = None;
        }
    }

    async fn authenticate(
        &self,
        credentials: Option<proto::hello::Credentials>,
    ) -> Result<SignedIn, Rejection> {
        use proto::hello::Credentials;
        let now = crate::clock::unix_seconds();
        match credentials
            .ok_or_else(|| Rejection::new(ErrorCode::InvalidRequest, "credentials are required"))?
        {
            Credentials::Guest(guest) => {
                if !self.context.allow_guests {
                    return Err(AuthError::GuestsDisabled.into());
                }
                Ok(SignedIn {
                    who: Authenticated::Guest {
                        name: DisplayName::parse(&guest.display_name)?,
                    },
                    session_token: String::new(),
                })
            }
            Credentials::Google(google) => {
                // Bounds the blocking threads that verification (and a key fetch) may occupy.
                let _permit = self
                    .context
                    .google_verifications
                    .acquire()
                    .await
                    .map_err(|_| Rejection::new(ErrorCode::Internal, "sign-in is unavailable"))?;
                let context = self.context.clone();
                let identity = tokio::task::spawn_blocking(move || match &context.google {
                    Some(verifier) => verifier.verify(&google.id_token),
                    None => Err(AuthError::GoogleDisabled),
                })
                .await
                .map_err(|_| Rejection::new(ErrorCode::Internal, "sign-in task failed"))??;
                let token = self.context.session_tokens.issue(
                    SessionKind::Google,
                    &identity.subject,
                    &identity.display_name,
                    now,
                );
                Ok(SignedIn {
                    who: Authenticated::Google {
                        subject: identity.subject,
                        name: identity.display_name,
                    },
                    session_token: token,
                })
            }
            Credentials::BotClient(bot) => {
                let name = self.context.bot_keys.authenticate(&bot.key)?.clone();
                let token = self.context.session_tokens.issue(
                    SessionKind::BotClient,
                    name.as_str(),
                    &name,
                    now,
                );
                Ok(SignedIn {
                    who: Authenticated::BotClient { name },
                    session_token: token,
                })
            }
            Credentials::Session(session) => {
                let claims = self.context.session_tokens.verify(&session.token, now)?;
                let who = match claims.kind {
                    SessionKind::Google => Authenticated::Google {
                        subject: claims.subject,
                        name: claims.display_name,
                    },
                    SessionKind::BotClient => {
                        if !self.context.bot_keys.is_configured(&claims.display_name) {
                            return Err(AuthError::BotRevoked.into());
                        }
                        Authenticated::BotClient {
                            name: claims.display_name,
                        }
                    }
                };
                Ok(SignedIn {
                    who,
                    session_token: session.token,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use futures_util::sink;
    use tokio::sync::mpsc;

    use super::*;

    fn recording_sink() -> (
        impl Sink<WsMessage> + Unpin,
        mpsc::UnboundedReceiver<WsMessage>,
    ) {
        let (sender, written) = mpsc::unbounded_channel();
        let sink = sink::unfold(sender, |sender, message: WsMessage| async move {
            if sender.send(message).is_err() {
                tracing::trace!("recording sink receiver dropped");
            }
            Ok::<_, Infallible>(sender)
        });
        (Box::pin(sink), written)
    }

    #[tokio::test(start_paused = true)]
    async fn writer_gives_up_on_a_socket_that_stops_accepting() {
        let (outbound, receiver) = Outbound::channel(ConnectionId::next());
        let stuck = Box::pin(sink::unfold((), |(), _message: WsMessage| async move {
            std::future::pending::<Result<(), Infallible>>().await
        }));
        let writer = tokio::spawn(write_loop(stuck, receiver));
        outbound.send(&codec::push(Message::Ack(proto::Ack {})));
        tokio::time::timeout(Limits::SEND_TIMEOUT * 3, writer)
            .await
            .expect("writer must stop after the send timeout")
            .unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn overloaded_connection_is_closed_without_flushing() {
        let (outbound, receiver) = Outbound::channel(ConnectionId::next());
        for _ in 0..Limits::OUTBOUND_QUEUE + 1 {
            outbound.send(&codec::push(Message::Ack(proto::Ack {})));
        }
        let (sink, mut written) = recording_sink();
        write_loop(sink, receiver).await;
        assert!(matches!(written.recv().await, Some(WsMessage::Close(_))));
        assert!(written.recv().await.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn normal_close_flushes_queued_messages_first() {
        let (outbound, receiver) = Outbound::channel(ConnectionId::next());
        outbound.send(&codec::push(Message::Ack(proto::Ack {})));
        outbound.close();
        let (sink, mut written) = recording_sink();
        write_loop(sink, receiver).await;
        assert!(matches!(written.recv().await, Some(WsMessage::Text(_))));
        assert!(matches!(written.recv().await, Some(WsMessage::Close(_))));
    }
}
