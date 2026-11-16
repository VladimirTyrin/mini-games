use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as ClientPayload;
use mg_protocol::proto::server_message::Message;
use tokio::sync::oneshot;

use crate::auth::{AuthError, DisplayName, SessionKind};
use crate::authenticated::Authenticated;
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
    messages: TokenBucket,
    chat: TokenBucket,
    /// Set after a fatal error (e.g. unsupported protocol): stop reading.
    finished: bool,
}

pub(crate) async fn run(context: Arc<ServerContext>, socket: WebSocket) {
    let id = ConnectionId::next();
    let (outbound, receiver) = Outbound::channel(id);
    let (sink, mut stream) = socket.split();
    let mut writer = tokio::spawn(write_loop(sink, receiver));
    let now = Instant::now();
    let mut connection = Connection {
        id,
        context,
        outbound,
        session: None,
        messages: TokenBucket::new(Limits::MESSAGE_BURST, Limits::MESSAGE_REFILL, now),
        chat: TokenBucket::new(Limits::CHAT_BURST, Limits::CHAT_REFILL, now),
        finished: false,
    };
    tracing::debug!(connection = %id, "connected");

    while !connection.finished {
        tokio::select! {
            next = tokio::time::timeout(Limits::IDLE_TIMEOUT, stream.next()) => match next {
                Ok(Some(Ok(WsMessage::Text(text)))) => connection.handle_text(text.as_str()).await,
                Ok(Some(Ok(WsMessage::Binary(_)))) => connection.reject(0, ErrorCode::InvalidRequest, "send JSON text frames"),
                Ok(Some(Ok(WsMessage::Ping(_) | WsMessage::Pong(_)))) => {}
                Ok(Some(Ok(WsMessage::Close(_)) | Err(_)) | None) => break,
                Err(_idle) => {
                    tracing::debug!(connection = %id, "idle timeout");
                    break;
                }
            },
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

async fn write_loop(
    mut sink: futures_util::stream::SplitSink<WebSocket, WsMessage>,
    mut outbound: OutboundReceiver,
) {
    loop {
        tokio::select! {
            // Drain queued messages before honouring a close request.
            biased;
            message = outbound.receiver.recv() => match message {
                Some(text) => {
                    if sink.send(WsMessage::Text(text)).await.is_err() {
                        return;
                    }
                }
                None => break,
            },
            () = outbound.close.notified() => {
                // An overloaded client gets no flush: that would block on the very client that
                // could not keep up.
                if !outbound.overloaded.load(Ordering::Relaxed) {
                    while let Ok(text) = outbound.receiver.try_recv() {
                        if sink.send(WsMessage::Text(text)).await.is_err() {
                            return;
                        }
                    }
                }
                break;
            }
        }
    }
    if let Err(error) = sink.send(WsMessage::Close(None)).await {
        tracing::trace!(%error, "close frame not sent");
    }
}

impl Connection {
    fn reject(&self, request_id: u32, code: ErrorCode, message: &str) {
        self.outbound.send(&codec::error(request_id, code, message));
    }

    async fn handle_text(&mut self, text: &str) {
        if !self.messages.try_take(Instant::now()) {
            return self.reject(0, ErrorCode::RateLimited, "slow down");
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
        if text.len() > Limits::MAX_MESSAGE_BYTES
            && !matches!(payload, ClientPayload::ShareReplay(_))
        {
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
                if matches!(payload, ClientPayload::SendChat(_))
                    && !self.chat.try_take(Instant::now())
                {
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
            request_id,
            reply,
        }));
        self.session = response.await.ok().flatten();
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
                    SessionKind::BotClient => Authenticated::BotClient {
                        name: claims.display_name,
                    },
                };
                Ok(SignedIn {
                    who,
                    session_token: session.token,
                })
            }
        }
    }
}
