#![allow(dead_code, unreachable_pub)]

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use mg_protocol::proto::client_message::Message as Request;
use mg_protocol::proto::server_message::Message as Reply;
use mg_protocol::{WireGame, proto};
use mg_server::{Config, Timing, serve};
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// Upper bound for waiting on an expected server message. Only limits waiting for something that
/// should arrive promptly; it is never used to make time pass.
pub(crate) const RECEIVE_TIMEOUT: Duration = Duration::from_secs(10);

/// `MG_BOT_KEYS` entry for bot `alpha` with key `secret`.
pub(crate) const ALPHA_BOT_KEY: &str =
    "alpha:2bb80d537b1da3e38bd30361aa855686bde0eacd7162fef6a25fe97bf527a25b";
pub(crate) const SESSION_SECRET: &str =
    "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

/// Production timing with the countdown and list coalescing shortened; tests shorten further
/// whatever wait their scenario is about, and then wait for the event, not for time.
pub(crate) fn quick_timing() -> Timing {
    Timing {
        resume_countdown: Duration::from_millis(50),
        lobby_list_coalesce: Duration::from_millis(20),
        ..Timing::default()
    }
}

pub(crate) struct TestServer {
    pub address: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
}

impl TestServer {
    pub(crate) async fn start() -> Self {
        Self::start_with(|_| {}).await
    }

    pub(crate) async fn start_with(configure: impl FnOnce(&mut Config)) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut config = <Config as clap::Parser>::parse_from(["mini_games_server"]);
        config.listen = address;
        configure(&mut config);
        let (shutdown, stopped) = oneshot::channel::<()>();
        tokio::spawn(async move {
            serve(config, listener, async move {
                let _closed = stopped.await;
            })
            .await
            .unwrap();
        });
        Self {
            address,
            shutdown: Some(shutdown),
        }
    }

    pub(crate) async fn client(&self) -> Client {
        self.try_client(None).await.expect("connect")
    }

    /// `real_ip` is sent as `X-Real-IP`, as the reverse proxy does.
    pub(crate) async fn try_client(
        &self,
        real_ip: Option<&str>,
    ) -> Result<Client, tokio_tungstenite::tungstenite::Error> {
        let mut request = format!("ws://{}/ws", self.address)
            .into_client_request()
            .unwrap();
        if let Some(ip) = real_ip {
            request
                .headers_mut()
                .insert("x-real-ip", ip.parse().unwrap());
        }
        let (socket, _) = tokio_tungstenite::connect_async(request).await?;
        Ok(Client {
            socket,
            next_request: 1,
        })
    }

    pub(crate) async fn guest(&self, name: &str) -> SignedInClient {
        let mut client = self.client().await;
        let welcome = client.hello_guest(name, "").await.expect("guest sign-in");
        SignedInClient { client, welcome }
    }

    pub(crate) fn stop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _sent = shutdown.send(());
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop();
    }
}

pub(crate) struct SignedInClient {
    pub client: Client,
    pub welcome: proto::Welcome,
}

pub(crate) struct Client {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_request: u32,
}

impl Client {
    pub(crate) async fn send(&mut self, message: Request) -> u32 {
        let request_id = self.next_request;
        self.next_request += 1;
        let envelope = proto::ClientMessage {
            request_id,
            message: Some(message),
        };
        let text = serde_json::to_string(&envelope).unwrap();
        self.socket
            .send(WsMessage::Text(text.into()))
            .await
            .unwrap();
        request_id
    }

    pub(crate) async fn send_raw(&mut self, text: &str) {
        self.socket
            .send(WsMessage::Text(text.into()))
            .await
            .unwrap();
    }

    pub(crate) async fn receive(&mut self) -> proto::ServerMessage {
        loop {
            let next = tokio::time::timeout(RECEIVE_TIMEOUT, self.socket.next())
                .await
                .expect("timed out waiting for a server message")
                .expect("connection closed")
                .expect("websocket error");
            match next {
                WsMessage::Text(text) => return serde_json::from_str(text.as_str()).unwrap(),
                WsMessage::Close(_) => panic!("server closed the connection"),
                _ => {}
            }
        }
    }

    /// Skips messages until `pick` returns something.
    pub(crate) async fn expect<T>(
        &mut self,
        mut pick: impl FnMut(&proto::ServerMessage) -> Option<T>,
    ) -> T {
        loop {
            let message = self.receive().await;
            if let Some(found) = pick(&message) {
                return found;
            }
        }
    }

    pub(crate) async fn reply_to(&mut self, request_id: u32) -> Reply {
        self.expect(|m| {
            (m.request_id == request_id)
                .then(|| m.message.clone())
                .flatten()
        })
        .await
    }

    pub(crate) async fn request(&mut self, message: Request) -> Reply {
        let request_id = self.send(message).await;
        self.reply_to(request_id).await
    }

    pub(crate) async fn hello_guest(
        &mut self,
        name: &str,
        resume_token: &str,
    ) -> Result<proto::Welcome, proto::ErrorResponse> {
        let credentials = proto::hello::Credentials::Guest(proto::GuestCredentials {
            display_name: name.into(),
        });
        self.hello(credentials, resume_token).await
    }

    pub(crate) async fn hello(
        &mut self,
        credentials: proto::hello::Credentials,
        resume_token: &str,
    ) -> Result<proto::Welcome, proto::ErrorResponse> {
        let hello = proto::Hello {
            protocol_version: 2,
            client_version: "test".into(),
            credentials: Some(credentials),
            resume_token: resume_token.into(),
        };
        match self.request(Request::Hello(hello)).await {
            Reply::Welcome(welcome) => Ok(welcome),
            Reply::Error(error) => Err(error),
            other => panic!("unexpected reply to hello: {other:?}"),
        }
    }

    pub(crate) async fn closed(&mut self) -> bool {
        loop {
            match tokio::time::timeout(RECEIVE_TIMEOUT, self.socket.next()).await {
                Ok(None | Some(Err(_)) | Some(Ok(WsMessage::Close(_)))) => return true,
                Ok(Some(Ok(_))) => {}
                Err(_) => return false,
            }
        }
    }
}

pub(crate) fn bot_credentials(key: &str) -> proto::hello::Credentials {
    proto::hello::Credentials::BotClient(proto::BotClientCredentials { key: key.into() })
}

/// Session credentials for a Google account, signed with `SESSION_SECRET` the way the server
/// signs them after verifying a Google ID token (which tests cannot obtain).
pub(crate) fn google_session(subject: &str, name: &str) -> proto::hello::Credentials {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use hmac::{Hmac, KeyInit, Mac};

    let secret: Vec<u8> = SESSION_SECRET
        .as_bytes()
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    let payload = serde_json::json!({
        "v": 2, "kind": "google", "sub": subject, "name": name, "exp": 9_999_999_999_u64,
    });
    let encoded = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap());
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(&secret).unwrap();
    mac.update(encoded.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    proto::hello::Credentials::Session(proto::SessionCredentials {
        token: format!("{encoded}.{signature}"),
    })
}

pub(crate) fn error_push(message: &proto::ServerMessage) -> Option<proto::ErrorResponse> {
    match &message.message {
        Some(Reply::Error(error)) => Some(error.clone()),
        _ => None,
    }
}

pub(crate) fn paused(message: &proto::ServerMessage) -> Option<proto::MatchPause> {
    match &message.message {
        Some(Reply::Paused(paused)) => paused.pause,
        _ => None,
    }
}

pub(crate) fn resumed(message: &proto::ServerMessage) -> Option<proto::MatchResumed> {
    match &message.message {
        Some(Reply::Resumed(resumed)) => Some(resumed.clone()),
        _ => None,
    }
}

pub(crate) fn catch_up(message: &proto::ServerMessage) -> Option<proto::MatchCatchUp> {
    match &message.message {
        Some(Reply::CatchUp(catch_up)) => Some(catch_up.clone()),
        _ => None,
    }
}

pub(crate) fn lobby_list(message: &proto::ServerMessage) -> Option<proto::LobbyList> {
    match &message.message {
        Some(Reply::LobbyList(list)) => Some(list.clone()),
        _ => None,
    }
}

/// Picks a frame that drops `slot`.
pub(crate) fn drops_slot(slot: u32) -> impl FnMut(&proto::ServerMessage) -> Option<()> {
    move |message| match &message.message {
        Some(Reply::Frame(frame))
            if frame
                .inputs
                .iter()
                .any(|input| input.input == Some(proto::frame_input::Input::DroppedSlot(slot))) =>
        {
            Some(())
        }
        _ => None,
    }
}

pub(crate) fn expect_ok(reply: Reply) {
    match reply {
        Reply::Ack(_) | Reply::LobbyState(_) => {}
        other => panic!("expected success, got {other:?}"),
    }
}

pub(crate) fn expect_error(reply: Reply, code: proto::ErrorCode) -> proto::ErrorResponse {
    match reply {
        Reply::Error(error) if error.code == code as i32 => error,
        other => panic!("expected {code:?}, got {other:?}"),
    }
}

pub(crate) fn tictactoe_settings(size: u32, win_count: u32) -> proto::GameSettings {
    mg_tictactoe::TicTacToe::settings_to_wire(&mg_tictactoe::RawSettings {
        field_width: size,
        field_height: size,
        win_count,
        first_player: mg_tictactoe::FirstPlayerMode::Host,
    })
}

/// Slow, wrapping snake on a large field: nobody dies during a test.
pub(crate) fn snake_settings() -> proto::GameSettings {
    mg_snake::Snake::settings_to_wire(&mg_snake::RawSettings {
        field_width: 100,
        field_height: 100,
        tick_interval_ms: 5000,
        ..mg_snake::RawSettings::default()
    })
}

pub(crate) fn turn_left(match_id: &str) -> Request {
    let command =
        mg_snake::Snake::command_to_wire(&mg_snake::Command::Turn(mg_snake::Direction::Left));
    Request::SubmitCommand(proto::SubmitCommand {
        match_id: match_id.into(),
        command: Some(command),
    })
}

pub(crate) async fn create_lobby(
    host: &mut Client,
    settings: proto::GameSettings,
    max_seats: u32,
) -> proto::LobbyState {
    let created = host
        .request(Request::CreateLobby(proto::CreateLobby {
            name: "test".into(),
            settings: Some(settings),
            max_seats,
        }))
        .await;
    match created {
        Reply::LobbyState(lobby) => lobby,
        other => panic!("create failed: {other:?}"),
    }
}

pub(crate) async fn join_ready(client: &mut Client, lobby_id: &str) {
    expect_ok(
        client
            .request(Request::JoinLobby(proto::JoinLobby {
                lobby_id: lobby_id.into(),
                as_observer: false,
            }))
            .await,
    );
    expect_ok(
        client
            .request(Request::SetReady(proto::SetReady { ready: true }))
            .await,
    );
}

pub(crate) async fn start(host: &mut Client) -> proto::MatchHeader {
    expect_ok(
        host.request(Request::StartMatch(proto::StartMatch {}))
            .await,
    );
    host.expect(match_started).await
}

pub(crate) fn place(match_id: &str, x: u32, y: u32) -> Request {
    let command = mg_tictactoe::TicTacToe::command_to_wire(&mg_tictactoe::Command::Place(
        mg_tictactoe::Point { x, y },
    ));
    Request::SubmitCommand(proto::SubmitCommand {
        match_id: match_id.into(),
        command: Some(command),
    })
}

pub(crate) fn lobby_state(message: &proto::ServerMessage) -> Option<proto::LobbyState> {
    match &message.message {
        Some(Reply::LobbyState(state)) => Some(state.clone()),
        _ => None,
    }
}

pub(crate) fn match_started(message: &proto::ServerMessage) -> Option<proto::MatchHeader> {
    match &message.message {
        Some(Reply::MatchStarted(header)) => Some(header.clone()),
        _ => None,
    }
}

pub(crate) fn match_ended(message: &proto::ServerMessage) -> Option<proto::MatchEnded> {
    match &message.message {
        Some(Reply::Ended(ended)) => Some(ended.clone()),
        _ => None,
    }
}

/// Host creates a 3x3 TicTacToe lobby, `guest` joins and readies, host starts. Returns the
/// match id.
pub(crate) async fn start_two_player_tictactoe(host: &mut Client, guest: &mut Client) -> String {
    let created = host
        .request(Request::CreateLobby(proto::CreateLobby {
            name: "test".into(),
            settings: Some(tictactoe_settings(3, 3)),
            max_seats: 0,
        }))
        .await;
    let Reply::LobbyState(lobby) = created else {
        panic!("create failed: {created:?}")
    };
    expect_ok(
        guest
            .request(Request::JoinLobby(proto::JoinLobby {
                lobby_id: lobby.lobby_id,
                as_observer: false,
            }))
            .await,
    );
    expect_ok(
        guest
            .request(Request::SetReady(proto::SetReady { ready: true }))
            .await,
    );
    expect_ok(
        host.request(Request::StartMatch(proto::StartMatch {}))
            .await,
    );
    let header = host.expect(match_started).await;
    guest.expect(match_started).await;
    header.match_id
}

/// A server on a runtime of its own that is dropped as soon as `serve` returns, the way the
/// binary's process exits then: whatever was not flushed by that moment is lost.
pub(crate) struct OwnRuntimeServer {
    pub address: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl OwnRuntimeServer {
    pub(crate) fn start() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let (shutdown, stopped) = oneshot::channel::<()>();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                let mut config = <Config as clap::Parser>::parse_from(["mini_games_server"]);
                config.listen = address;
                serve(config, listener, async move {
                    let _closed = stopped.await;
                })
                .await
                .unwrap();
            });
            drop(runtime);
        });
        Self {
            address,
            shutdown: Some(shutdown),
            thread: Some(thread),
        }
    }

    pub(crate) async fn client(&self) -> Client {
        let (socket, _) = tokio_tungstenite::connect_async(format!("ws://{}/ws", self.address))
            .await
            .unwrap();
        Client {
            socket,
            next_request: 1,
        }
    }

    /// Stops the server and waits until its runtime is gone.
    pub(crate) async fn stop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _sent = shutdown.send(());
        }
        if let Some(thread) = self.thread.take() {
            tokio::task::spawn_blocking(move || thread.join().unwrap())
                .await
                .unwrap();
        }
    }
}
