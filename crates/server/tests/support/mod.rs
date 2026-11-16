#![allow(dead_code)]

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use mg_protocol::proto::client_message::Message as Request;
use mg_protocol::proto::server_message::Message as Reply;
use mg_protocol::{WireGame, proto};
use mg_server::{Config, serve};
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// Upper bound for waiting on an expected server message. Only limits waiting for something that
/// should arrive promptly; it is never used to make time pass.
pub const RECEIVE_TIMEOUT: Duration = Duration::from_secs(10);

pub struct TestServer {
    pub address: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
}

impl TestServer {
    pub async fn start() -> Self {
        Self::start_with(|_| {}).await
    }

    pub async fn start_with(configure: impl FnOnce(&mut Config)) -> Self {
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

    pub async fn client(&self) -> Client {
        let url = format!("ws://{}/ws", self.address);
        let (socket, _) = tokio_tungstenite::connect_async(url).await.unwrap();
        Client {
            socket,
            next_request: 1,
        }
    }

    pub async fn guest(&self, name: &str) -> SignedInClient {
        let mut client = self.client().await;
        let welcome = client.hello_guest(name, "").await.expect("guest sign-in");
        SignedInClient { client, welcome }
    }

    pub fn stop(&mut self) {
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

pub struct SignedInClient {
    pub client: Client,
    pub welcome: proto::Welcome,
}

pub struct Client {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_request: u32,
}

impl Client {
    pub async fn send(&mut self, message: Request) -> u32 {
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

    pub async fn send_raw(&mut self, text: &str) {
        self.socket
            .send(WsMessage::Text(text.into()))
            .await
            .unwrap();
    }

    pub async fn receive(&mut self) -> proto::ServerMessage {
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
    pub async fn expect<T>(
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

    pub async fn reply_to(&mut self, request_id: u32) -> Reply {
        self.expect(|m| {
            (m.request_id == request_id)
                .then(|| m.message.clone())
                .flatten()
        })
        .await
    }

    pub async fn request(&mut self, message: Request) -> Reply {
        let request_id = self.send(message).await;
        self.reply_to(request_id).await
    }

    pub async fn hello_guest(
        &mut self,
        name: &str,
        resume_token: &str,
    ) -> Result<proto::Welcome, proto::ErrorResponse> {
        let hello = proto::Hello {
            protocol_version: 2,
            client_version: "test".into(),
            credentials: Some(proto::hello::Credentials::Guest(proto::GuestCredentials {
                display_name: name.into(),
            })),
            resume_token: resume_token.into(),
        };
        match self.request(Request::Hello(hello)).await {
            Reply::Welcome(welcome) => Ok(welcome),
            Reply::Error(error) => Err(error),
            other => panic!("unexpected reply to hello: {other:?}"),
        }
    }

    pub async fn closed(&mut self) -> bool {
        loop {
            match tokio::time::timeout(RECEIVE_TIMEOUT, self.socket.next()).await {
                Ok(None | Some(Err(_)) | Some(Ok(WsMessage::Close(_)))) => return true,
                Ok(Some(Ok(_))) => {}
                Err(_) => return false,
            }
        }
    }
}

pub fn expect_ok(reply: Reply) {
    match reply {
        Reply::Ack(_) | Reply::LobbyState(_) => {}
        other => panic!("expected success, got {other:?}"),
    }
}

pub fn expect_error(reply: Reply, code: proto::ErrorCode) -> proto::ErrorResponse {
    match reply {
        Reply::Error(error) if error.code == code as i32 => error,
        other => panic!("expected {code:?}, got {other:?}"),
    }
}

pub fn tictactoe_settings(size: u32, win_count: u32) -> proto::GameSettings {
    mg_tictactoe::TicTacToe::settings_to_wire(&mg_tictactoe::Settings {
        field_width: size,
        field_height: size,
        win_count,
        first_player: mg_tictactoe::FirstPlayerMode::Host,
    })
}

pub fn place(match_id: &str, x: u32, y: u32) -> Request {
    let command = mg_tictactoe::TicTacToe::command_to_wire(&mg_tictactoe::Command::Place(
        mg_tictactoe::Point { x, y },
    ));
    Request::SubmitCommand(proto::SubmitCommand {
        match_id: match_id.into(),
        command: Some(command),
    })
}

pub fn lobby_state(message: &proto::ServerMessage) -> Option<proto::LobbyState> {
    match &message.message {
        Some(Reply::LobbyState(state)) => Some(state.clone()),
        _ => None,
    }
}

pub fn match_started(message: &proto::ServerMessage) -> Option<proto::MatchHeader> {
    match &message.message {
        Some(Reply::MatchStarted(header)) => Some(header.clone()),
        _ => None,
    }
}

pub fn match_ended(message: &proto::ServerMessage) -> Option<proto::MatchEnded> {
    match &message.message {
        Some(Reply::Ended(ended)) => Some(ended.clone()),
        _ => None,
    }
}

/// Host creates a 3x3 TicTacToe lobby, `guest` joins and readies, host starts. Returns the
/// match id.
pub async fn start_two_player_tictactoe(host: &mut Client, guest: &mut Client) -> String {
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
