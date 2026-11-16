//! Connections, sign-in and server-wide limits.

mod support;

use std::time::Duration;

use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as Request;
use mg_protocol::proto::server_message::Message as Reply;
use support::*;

#[tokio::test]
async fn welcome_carries_server_info() {
    let server = TestServer::start().await;
    let alice = server.guest("Alice").await;
    let info = alice.welcome.server_info.expect("server info in Welcome");
    assert_eq!(info.min_protocol_version, 2);
    assert!(info.guests_allowed);
}

#[tokio::test]
async fn second_sign_in_of_an_account_tells_the_first_not_to_reconnect() {
    let server = TestServer::start_with(|config| config.bot_keys = ALPHA_BOT_KEY.into()).await;
    let mut first = server.client().await;
    first.hello(bot_credentials("secret"), "").await.unwrap();
    let mut second = server.client().await;
    second.hello(bot_credentials("secret"), "").await.unwrap();

    let error = first.expect(error_push).await;
    assert_eq!(error.code, proto::ErrorCode::SignedInElsewhere as i32);
    assert!(first.closed().await);
}

#[tokio::test]
async fn bot_session_token_stops_working_once_the_key_is_removed() {
    let configure = |bot_keys: &'static str| {
        move |config: &mut mg_server::Config| {
            config.bot_keys = bot_keys.into();
            config.session_secret = Some(SESSION_SECRET.into());
        }
    };
    let with_key = TestServer::start_with(configure(ALPHA_BOT_KEY)).await;
    let mut bot = with_key.client().await;
    let token = bot
        .hello(bot_credentials("secret"), "")
        .await
        .unwrap()
        .session_token;
    let session = || {
        proto::hello::Credentials::Session(proto::SessionCredentials {
            token: token.clone(),
        })
    };

    let still_configured = TestServer::start_with(configure(ALPHA_BOT_KEY)).await;
    let mut client = still_configured.client().await;
    assert!(client.hello(session(), "").await.is_ok());

    let key_removed = TestServer::start_with(configure("")).await;
    let mut client = key_removed.client().await;
    let error = client.hello(session(), "").await.unwrap_err();
    assert_eq!(error.code, proto::ErrorCode::SignInFailed as i32);
}

#[tokio::test]
async fn a_connection_that_never_says_hello_is_closed() {
    let server = TestServer::start_with(|config| {
        config.timing.hello_timeout = Duration::from_millis(100);
    })
    .await;
    let mut silent = server.client().await;
    let Reply::ServerInfo(_) = silent
        .request(Request::ServerInfo(proto::ServerInfoRequest {}))
        .await
    else {
        panic!("server info is answered before Hello");
    };
    assert!(silent.closed().await);

    // Signing in in time keeps the connection.
    let mut alice = server.guest("Alice").await.client;
    let Reply::LobbyList(_) = alice
        .request(Request::ListLobbies(proto::ListLobbies {}))
        .await
    else {
        panic!("signed-in connection must stay open");
    };
}

#[tokio::test]
async fn connections_per_client_address_are_capped() {
    let server = TestServer::start_with(|config| config.max_connections_per_ip = 2).await;
    let _first = server.client().await;
    let _second = server.client().await;
    assert!(server.try_client(None).await.is_err());
    // Behind the proxy (loopback peer) the address comes from X-Real-IP.
    assert!(server.try_client(Some("203.0.113.7")).await.is_ok());
}

#[tokio::test]
async fn oversized_messages_before_sign_in_are_refused_unread_and_closed() {
    let server = TestServer::start().await;
    let mut anonymous = server.client().await;
    anonymous
        .send_raw(&format!("{{{}", "x".repeat(70_000)))
        .await;
    let error = anonymous.expect(error_push).await;
    assert_eq!(error.code, proto::ErrorCode::InvalidRequest as i32);
    assert_eq!(error.message, "message too large");
    assert!(anonymous.closed().await);
}

#[tokio::test]
async fn shutdown_reaches_clients_before_the_server_exits() {
    let mut server = OwnRuntimeServer::start();
    let mut alice = server.client().await;
    alice.hello_guest("Alice", "").await.unwrap();
    server.stop().await;
    alice
        .expect(|m| match &m.message {
            Some(Reply::ShuttingDown(_)) => Some(()),
            _ => None,
        })
        .await;
}
