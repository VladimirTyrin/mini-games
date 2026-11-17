mod support;

use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as Request;
use mg_protocol::proto::server_message::Message as Reply;
use support::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// What deploy.sh does before a restart: `POST /admin/drain`, answered with the matches running.
async fn drain(server: &TestServer) -> String {
    let mut http = tokio::net::TcpStream::connect(server.address)
        .await
        .unwrap();
    http.write_all(
        b"POST /admin/drain HTTP/1.1\r\nHost: test\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    )
    .await
    .unwrap();
    let mut response = String::new();
    http.read_to_string(&mut response).await.unwrap();
    response
}

fn body(response: &str) -> &str {
    response
        .split_once("\r\n\r\n")
        .map_or("", |(_, body)| body)
        .trim()
}

fn server_updating(message: &proto::ServerMessage) -> Option<()> {
    matches!(message.message, Some(Reply::ServerUpdating(_))).then_some(())
}

#[tokio::test]
async fn before_a_restart_running_matches_play_on_and_new_ones_wait() {
    let server = TestServer::start().await;
    let mut ada = server.guest("Ada").await.client;
    let mut bob = server.guest("Bob").await.client;
    let match_id = start_two_player_tictactoe(&mut ada, &mut bob).await;
    let mut cid = server.guest("Cid").await.client;
    let mut dan = server.guest("Dan").await.client;
    let lobby = create_lobby(&mut cid, tictactoe_settings(3, 3), 2).await;
    join_ready(&mut dan, &lobby.lobby_id).await;

    let response = drain(&server).await;

    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert_eq!(body(&response), "1");
    cid.expect(server_updating).await;
    expect_error(
        cid.request(Request::StartMatch(proto::StartMatch {})).await,
        proto::ErrorCode::ServerUpdating,
    );
    expect_ok(ada.request(place(&match_id, 0, 0)).await);
    let mut late = server.guest("Eve").await.client;
    late.expect(server_updating).await;
}

#[tokio::test]
async fn drain_reports_zero_once_the_last_match_is_over() {
    let server = TestServer::start().await;
    let mut ada = server.guest("Ada").await.client;
    let mut bob = server.guest("Bob").await.client;
    let match_id = start_two_player_tictactoe(&mut ada, &mut bob).await;
    assert_eq!(body(&drain(&server).await), "1");

    // Ada, who moves first, wins on the top row.
    for (mover, x, y) in [(0, 0, 0), (1, 0, 1), (0, 1, 0), (1, 1, 1), (0, 2, 0)] {
        let client = if mover == 0 { &mut ada } else { &mut bob };
        expect_ok(client.request(place(&match_id, x, y)).await);
    }
    ada.expect(match_ended).await;

    assert_eq!(body(&drain(&server).await), "0");
}
