use std::collections::VecDeque;
use std::time::Duration;

use mg_core::Slot;
use mg_protocol::proto::client_message::Message as Request;
use mg_protocol::proto::server_message::Message as Reply;
use mg_protocol::{proto, slot_from_wire};
use tokio::time::{Instant, Interval, MissedTickBehavior};

use crate::PROTOCOL_VERSION;
use crate::bot_config::BotConfig;
use crate::connection::Connection;
use crate::lobby_setup::LobbySetup;
use crate::resumption::Resumption;
use crate::sdk_error::SdkError;
use crate::server_error::ServerError;

/// The server closes connections silent for 45 s; clients ping every 15 s.
const PING_EVERY: Duration = Duration::from_secs(15);

/// A signed-in bot connection. Requests wait for their reply; pushes that arrive meanwhile are
/// kept and handed out by [`BotSession::next_message`] in order, so nothing is lost.
#[derive(Debug)]
pub struct BotSession {
    config: BotConfig,
    connection: Connection,
    welcome: proto::Welcome,
    next_request_id: u32,
    pending: VecDeque<proto::ServerMessage>,
    ping: Interval,
    pub(crate) reconnects: u32,
}

impl BotSession {
    /// Connects and signs in with the bot key.
    pub async fn connect(config: BotConfig) -> Result<Self, SdkError> {
        let (connection, welcome) = sign_in(&config, "").await?;
        Ok(Self {
            config,
            connection,
            welcome,
            // 1 was the Hello.
            next_request_id: 2,
            pending: VecDeque::new(),
            ping: ping_interval(),
            reconnects: 0,
        })
    }

    /// The latest `Welcome` (identity, session and resume tokens).
    pub fn welcome(&self) -> &proto::Welcome {
        &self.welcome
    }

    /// Successful reconnects since [`BotSession::connect`].
    pub fn reconnects(&self) -> u32 {
        self.reconnects
    }

    /// Sends without waiting for the reply; returns the request id the reply will carry.
    pub async fn send(&mut self, message: Request) -> Result<u32, SdkError> {
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.checked_add(1).unwrap_or(1);
        self.connection
            .send(&proto::ClientMessage {
                request_id,
                message: Some(message),
            })
            .await?;
        Ok(request_id)
    }

    /// Sends and waits for the reply; an `ErrorResponse` becomes [`SdkError::Server`].
    pub async fn request(&mut self, message: Request) -> Result<Reply, SdkError> {
        let request_id = self.send(message).await?;
        loop {
            let message = self.read().await?;
            if message.request_id != request_id {
                self.pending.push_back(message);
                continue;
            }
            return match message.message {
                Some(Reply::Error(error)) => Err(SdkError::Server(ServerError::from_wire(error))),
                Some(reply) => Ok(reply),
                None => Err(SdkError::UnexpectedReply {
                    expected: "a reply",
                    got: "an empty message".to_owned(),
                }),
            };
        }
    }

    /// The next server message (replies to [`BotSession::send`] and pushes). Keeps the
    /// connection alive with pings while waiting. Pushes that end the session are errors.
    pub async fn next_message(&mut self) -> Result<proto::ServerMessage, SdkError> {
        let message = match self.pending.pop_front() {
            Some(message) => message,
            None => self.read().await?,
        };
        match &message.message {
            Some(Reply::Error(error))
                if error.code == proto::ErrorCode::SignedInElsewhere as i32 =>
            {
                Err(SdkError::SignedInElsewhere)
            }
            Some(Reply::ShuttingDown(_)) => Err(SdkError::ServerShuttingDown),
            _ => Ok(message),
        }
    }

    /// Like [`BotSession::next_message`], but resumes the session when the connection drops.
    pub async fn next_message_resuming(&mut self) -> Result<proto::ServerMessage, SdkError> {
        loop {
            match self.next_message().await {
                Err(error) if error.is_connection_loss() => self.resume().await?,
                other => return other,
            }
        }
    }

    /// Drops the connection (if still open) and signs in again with the resume token, following
    /// the session's [`crate::ReconnectPolicy`].
    pub async fn reconnect(&mut self) -> Result<Resumption, SdkError> {
        let policy = self.config.reconnect;
        let mut last_failure = SdkError::ConnectionClosed;
        for retry in 0..policy.attempts {
            tokio::time::sleep(policy.delay_before(retry)).await;
            match sign_in(&self.config, &self.welcome.resume_token).await {
                Ok((connection, welcome)) => {
                    let resumed = welcome.resumed;
                    self.connection = connection;
                    self.welcome = welcome;
                    self.ping = ping_interval();
                    self.reconnects += 1;
                    return Ok(if resumed {
                        Resumption::Resumed
                    } else {
                        Resumption::NewSession
                    });
                }
                Err(error) if error.is_connection_loss() => last_failure = error,
                Err(error) => return Err(error),
            }
        }
        Err(SdkError::ReconnectFailed {
            attempts: policy.attempts,
            last: Box::new(last_failure),
        })
    }

    /// Reconnects and insists on getting the same seat back.
    pub(crate) async fn resume(&mut self) -> Result<(), SdkError> {
        match self.reconnect().await? {
            Resumption::Resumed => Ok(()),
            Resumption::NewSession => Err(SdkError::SeatLost),
        }
    }

    /// Creates a lobby, seats the built-in bots, waits until every seat is taken and ready, and
    /// starts the match.
    pub async fn host_match(&mut self, setup: &LobbySetup) -> Result<proto::MatchHeader, SdkError> {
        let lobby = self.create_lobby(setup).await?;
        self.start_when_ready(lobby).await
    }

    /// For the host: waits until every seat is taken and ready (`lobby` is the latest state the
    /// caller has), then starts the match.
    pub async fn start_when_ready(
        &mut self,
        mut lobby: proto::LobbyState,
    ) -> Result<proto::MatchHeader, SdkError> {
        while !is_ready_to_start(&lobby) {
            lobby = self.next_lobby_state().await?;
        }
        self.request(Request::StartMatch(proto::StartMatch {}))
            .await?;
        self.wait_for_match().await
    }

    /// Creates a lobby and seats the built-in bots. Returns the lobby as created (before the
    /// bots); later states arrive as pushes.
    pub async fn create_lobby(
        &mut self,
        setup: &LobbySetup,
    ) -> Result<proto::LobbyState, SdkError> {
        let reply = self
            .request(Request::CreateLobby(proto::CreateLobby {
                name: setup.name.clone(),
                settings: Some(setup.settings),
                max_seats: setup.max_seats,
            }))
            .await?;
        let lobby = expect_lobby_state(reply)?;
        for bot in &setup.built_in_bots {
            self.request(Request::AddBot(proto::AddBot {
                bot_kind: Some(*bot),
            }))
            .await?;
        }
        Ok(lobby)
    }

    /// Takes a seat in someone's lobby, marks it ready and waits for the host to start.
    pub async fn join_match(&mut self, lobby_id: &str) -> Result<proto::MatchHeader, SdkError> {
        self.request(Request::JoinLobby(proto::JoinLobby {
            lobby_id: lobby_id.to_owned(),
            as_observer: false,
        }))
        .await?;
        self.request(Request::SetReady(proto::SetReady { ready: true }))
            .await?;
        self.wait_for_match().await
    }

    pub async fn wait_for_match(&mut self) -> Result<proto::MatchHeader, SdkError> {
        loop {
            match self.next_message_resuming().await?.message {
                Some(Reply::MatchStarted(header)) => return Ok(header),
                Some(Reply::LobbyClosed(_)) => return Err(SdkError::LobbyClosed),
                _ => {}
            }
        }
    }

    pub async fn leave_lobby(&mut self) -> Result<(), SdkError> {
        self.request(Request::LeaveLobby(proto::LeaveLobby {}))
            .await
            .map(drop)
    }

    /// This bot's seat in a match, `None` when it only watches.
    pub fn slot_in(&self, header: &proto::MatchHeader) -> Option<Slot> {
        let me = self.welcome.identity.as_ref()?;
        header
            .seats
            .iter()
            .find(|seat| {
                seat.player
                    .as_ref()
                    .is_some_and(|player| player.kind == me.kind && player.id == me.id)
            })
            .and_then(|seat| slot_from_wire(seat.slot).ok())
    }

    async fn next_lobby_state(&mut self) -> Result<proto::LobbyState, SdkError> {
        loop {
            match self.next_message_resuming().await?.message {
                Some(Reply::LobbyState(lobby)) => return Ok(lobby),
                Some(Reply::LobbyClosed(_)) => return Err(SdkError::LobbyClosed),
                _ => {}
            }
        }
    }

    async fn read(&mut self) -> Result<proto::ServerMessage, SdkError> {
        loop {
            tokio::select! {
                message = self.connection.receive() => return message,
                _ = self.ping.tick() => {
                    let ping = proto::ClientMessage {
                        request_id: 0,
                        message: Some(Request::Ping(proto::Ping { client_time_ms: 0 })),
                    };
                    self.connection.send(&ping).await?;
                }
            }
        }
    }
}

fn ping_interval() -> Interval {
    let mut interval = tokio::time::interval_at(Instant::now() + PING_EVERY, PING_EVERY);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    interval
}

async fn sign_in(
    config: &BotConfig,
    resume_token: &str,
) -> Result<(Connection, proto::Welcome), SdkError> {
    const HELLO_REQUEST: u32 = 1;
    let mut connection = Connection::open(&config.url).await?;
    let hello = proto::Hello {
        protocol_version: PROTOCOL_VERSION,
        client_version: config.client_version.clone(),
        credentials: Some(proto::hello::Credentials::BotClient(
            proto::BotClientCredentials {
                key: config.bot_key.clone(),
            },
        )),
        resume_token: resume_token.to_owned(),
    };
    connection
        .send(&proto::ClientMessage {
            request_id: HELLO_REQUEST,
            message: Some(Request::Hello(hello)),
        })
        .await?;
    loop {
        let message = connection.receive().await?;
        if message.request_id != HELLO_REQUEST {
            continue;
        }
        return match message.message {
            Some(Reply::Welcome(welcome)) => Ok((connection, welcome)),
            Some(Reply::Error(error)) => Err(SdkError::Server(ServerError::from_wire(error))),
            other => Err(SdkError::UnexpectedReply {
                expected: "Welcome",
                got: format!("{other:?}"),
            }),
        };
    }
}

fn expect_lobby_state(reply: Reply) -> Result<proto::LobbyState, SdkError> {
    match reply {
        Reply::LobbyState(lobby) => Ok(lobby),
        other => Err(SdkError::UnexpectedReply {
            expected: "LobbyState",
            got: format!("{other:?}"),
        }),
    }
}

/// Every seat taken, and every seat but the host's ready (built-in bots always are).
fn is_ready_to_start(lobby: &proto::LobbyState) -> bool {
    let seats = u32::try_from(lobby.seats.len()).unwrap_or(u32::MAX);
    seats >= lobby.max_seats
        && lobby
            .seats
            .iter()
            .skip(1)
            .all(|seat| seat.ready || seat.bot_kind.is_some())
}
