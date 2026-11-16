mod support;

use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as Request;
use mg_protocol::proto::server_message::Message as Reply;
use support::*;

const VAPID_KEY: &str = "c9f58f89813e9f8e872e71f42aa64e1757c9254dcc62b72ddc010bb4043ea11c";

async fn web_push_server() -> TestServer {
    TestServer::start_with(|config| {
        config.vapid_private_key = VAPID_KEY.to_owned();
        config.vapid_subject = "https://example.org".to_owned();
    })
    .await
}

/// A subscription as a browser reports it (keys from RFC 8291's example).
fn web_push_target(endpoint: &str) -> proto::PushTarget {
    proto::PushTarget {
        target: Some(proto::push_target::Target::WebPush(
            proto::WebPushSubscription {
                endpoint: endpoint.to_owned(),
                p256dh: "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4".to_owned(),
                auth: "BTBZMqHH6r4Tts7J_aSIgg".to_owned(),
            },
        )),
    }
}

/// The longest delay, so the accepted test never reaches the network before the test ends.
fn test_push(target: proto::PushTarget) -> Request {
    Request::RequestTestPush(proto::RequestTestPush {
        target: Some(target),
        delay_seconds: 60,
    })
}

async fn server_info(server: &TestServer) -> proto::ServerInfo {
    let mut client = server.client().await;
    let Reply::ServerInfo(info) = client
        .request(Request::ServerInfo(proto::ServerInfoRequest {}))
        .await
    else {
        panic!("expected server info");
    };
    info
}

#[tokio::test]
async fn server_info_names_the_web_push_key_only_when_web_push_is_configured() {
    let without = TestServer::start().await;
    assert_eq!(server_info(&without).await.web_push_public_key, "");

    let with = web_push_server().await;
    let key = server_info(&with).await.web_push_public_key;
    // Uncompressed P-256 point (65 bytes), base64url without padding.
    assert_eq!(key.len(), 87);
    assert!(key.starts_with('B'));
}

#[tokio::test]
async fn test_push_is_refused_for_a_kind_of_device_the_server_cannot_reach() {
    let server = TestServer::start().await;
    let mut alice = server.guest("Alice").await.client;

    let reply = alice
        .request(test_push(web_push_target("https://web.push.apple.com/abc")))
        .await;

    expect_error(reply, proto::ErrorCode::PushUnavailable);
}

#[tokio::test]
async fn test_push_is_refused_for_an_endpoint_outside_the_push_services() {
    let server = web_push_server().await;
    let mut alice = server.guest("Alice").await.client;

    let reply = alice
        .request(test_push(web_push_target("https://127.0.0.1:5000/ws")))
        .await;

    expect_error(reply, proto::ErrorCode::InvalidRequest);
}

#[tokio::test]
async fn test_push_delay_is_capped() {
    let server = web_push_server().await;
    let mut alice = server.guest("Alice").await.client;

    let reply = alice
        .request(Request::RequestTestPush(proto::RequestTestPush {
            target: Some(web_push_target("https://web.push.apple.com/abc")),
            delay_seconds: 61,
        }))
        .await;

    expect_error(reply, proto::ErrorCode::InvalidRequest);
}

#[tokio::test]
async fn one_test_push_is_accepted_per_cooldown() {
    let server = web_push_server().await;
    let mut alice = server.guest("Alice").await.client;

    let first = alice
        .request(test_push(web_push_target("https://web.push.apple.com/abc")))
        .await;
    let second = alice
        .request(test_push(web_push_target("https://web.push.apple.com/abc")))
        .await;

    assert!(matches!(first, Reply::Ack(_)), "{first:?}");
    expect_error(second, proto::ErrorCode::RateLimited);
}

#[tokio::test]
async fn test_push_needs_sign_in() {
    let server = web_push_server().await;
    let mut anonymous = server.client().await;

    let reply = anonymous
        .request(test_push(web_push_target("https://web.push.apple.com/abc")))
        .await;

    expect_error(reply, proto::ErrorCode::NotSignedIn);
}

#[tokio::test]
async fn a_web_push_key_without_a_subject_leaves_web_push_off_instead_of_failing() {
    let server = TestServer::start_with(|config| {
        config.vapid_private_key = VAPID_KEY.to_owned();
    })
    .await;

    assert_eq!(server_info(&server).await.web_push_public_key, "");
}
