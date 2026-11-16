mod support;

use mg_core::Viewer;
use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as Request;
use mg_protocol::proto::server_message::Message as Reply;
use prost::Message as _;
use support::*;

#[tokio::test]
async fn server_info_is_available_before_sign_in() {
    let server = TestServer::start_with(|config| config.allow_guests = false).await;
    let mut client = server.client().await;
    let Reply::ServerInfo(info) = client
        .request(Request::ServerInfo(proto::ServerInfoRequest {}))
        .await
    else {
        panic!("expected server info");
    };
    assert!(!info.guests_allowed);
    assert_eq!(info.min_protocol_version, 2);
    let error = client.hello_guest("Alice", "").await.unwrap_err();
    assert_eq!(error.code, proto::ErrorCode::GuestsDisabled as i32);
}

#[tokio::test]
async fn unsupported_protocol_is_refused_and_closed() {
    let server = TestServer::start().await;
    let mut client = server.client().await;
    let hello = proto::Hello {
        protocol_version: 1,
        client_version: "old".into(),
        credentials: Some(proto::hello::Credentials::Guest(proto::GuestCredentials {
            display_name: "Old".into(),
        })),
        resume_token: String::new(),
    };
    expect_error(
        client.request(Request::Hello(hello)).await,
        proto::ErrorCode::ProtocolUnsupported,
    );
    assert!(client.closed().await);
}

#[tokio::test]
async fn requests_need_sign_in_and_names_are_unique() {
    let server = TestServer::start().await;
    let mut anonymous = server.client().await;
    expect_error(
        anonymous
            .request(Request::ListLobbies(proto::ListLobbies {}))
            .await,
        proto::ErrorCode::NotSignedIn,
    );
    let _alice = server.guest("Alice").await;
    let error = anonymous.hello_guest("alice", "").await.unwrap_err();
    assert_eq!(error.code, proto::ErrorCode::NameTaken as i32);
}

#[tokio::test]
async fn malformed_and_oversized_messages_get_errors_without_closing() {
    let server = TestServer::start().await;
    let mut alice = server.guest("Alice").await.client;
    alice.send_raw("{not json").await;
    let error = alice
        .expect(|m| match &m.message {
            Some(Reply::Error(e)) => Some(e.clone()),
            _ => None,
        })
        .await;
    assert_eq!(error.code, proto::ErrorCode::InvalidRequest as i32);
    let huge_chat = format!(
        r#"{{"requestId":9,"sendChat":{{"scope":"CHAT_SCOPE_LOBBY_LIST","text":"{}"}}}}"#,
        "x".repeat(70_000)
    );
    alice.send_raw(&huge_chat).await;
    expect_error(alice.reply_to(9).await, proto::ErrorCode::InvalidRequest);
    let Reply::LobbyList(_) = alice
        .request(Request::ListLobbies(proto::ListLobbies {}))
        .await
    else {
        panic!("connection should still work");
    };
}

#[tokio::test]
async fn invalid_settings_are_rejected() {
    let server = TestServer::start().await;
    let mut alice = server.guest("Alice").await.client;
    let reply = alice
        .request(Request::CreateLobby(proto::CreateLobby {
            name: String::new(),
            settings: Some(tictactoe_settings(100, 5)),
            max_seats: 0,
        }))
        .await;
    expect_error(reply, proto::ErrorCode::InvalidSettings);
}

#[tokio::test]
async fn two_players_play_tictactoe_to_the_end_and_download_a_valid_replay() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    let match_id = start_two_player_tictactoe(&mut host, &mut guest).await;

    // Host is slot 0 and plays X first ("Host" first-player mode): X takes column x = 0.
    for (x, y) in [(0, 0), (0, 1), (0, 2)] {
        expect_ok(host.request(place(&match_id, x, y)).await);
        if y < 2 {
            expect_ok(guest.request(place(&match_id, 1, y)).await);
        }
    }
    let ended = host.expect(match_ended).await;
    let result = ended.result.unwrap();
    assert_eq!(result.kind, proto::OutcomeKind::Winner as i32);
    assert_eq!(result.winner_slots, vec![0]);
    guest.expect(match_ended).await;

    let lobby = host
        .expect(|m| lobby_state(m).filter(|l| l.phase == proto::LobbyPhase::PostMatch as i32))
        .await;
    assert_eq!(lobby.seats.len(), 2);

    let Reply::ReplayData(replay) = host
        .request(Request::RequestReplay(proto::RequestReplay { match_id }))
        .await
    else {
        panic!("expected replay data");
    };
    let mg_registry::LoadedReplay { mut mirror, log } =
        mg_registry::load_replay_file(&replay.replay_file).unwrap();
    mirror.apply_compact_log(&log, log.end()).unwrap();
    assert!(mirror.status().is_finished());
    assert!(mirror.view(Viewer::Spectator).game.is_some());
}

#[tokio::test]
async fn moves_out_of_turn_and_from_observers_are_rejected() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    let match_id = start_two_player_tictactoe(&mut host, &mut guest).await;
    expect_error(
        guest.request(place(&match_id, 1, 1)).await,
        proto::ErrorCode::CommandRejected,
    );
    expect_error(
        host.request(place("0000000000000000", 1, 1)).await,
        proto::ErrorCode::MatchNotFound,
    );
}

#[tokio::test]
async fn bot_opponent_answers_every_move() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let Reply::LobbyState(_) = host
        .request(Request::CreateLobby(proto::CreateLobby {
            name: "bots".into(),
            settings: Some(tictactoe_settings(3, 3)),
            max_seats: 2,
        }))
        .await
    else {
        panic!("create failed");
    };
    let minimax = proto::BotKind {
        game: Some(proto::bot_kind::Game::Tictactoe(
            proto::TicTacToeBotKind::Minimax as i32,
        )),
    };
    expect_ok(
        host.request(Request::AddBot(proto::AddBot {
            bot_kind: Some(minimax),
        }))
        .await,
    );
    expect_ok(
        host.request(Request::StartMatch(proto::StartMatch {}))
            .await,
    );
    let header = host.expect(match_started).await;
    // The host plays the centre; the minimax bot (slot 1) must answer with a frame of its own.
    expect_ok(host.request(place(&header.match_id, 1, 1)).await);
    host.expect(|m| match &m.message {
        Some(Reply::Frame(frame)) if frame.inputs.iter().any(is_slot_1_command) => Some(()),
        _ => None,
    })
    .await;
}

fn is_slot_1_command(input: &proto::FrameInput) -> bool {
    matches!(&input.input, Some(proto::frame_input::Input::Command(command)) if command.slot == 1)
}

#[tokio::test]
async fn disconnect_pauses_and_resume_token_restores_the_seat() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let guest = server.guest("Guest").await;
    let resume_token = guest.welcome.resume_token.clone();
    let mut guest = guest.client;
    let match_id = start_two_player_tictactoe(&mut host, &mut guest).await;
    drop(guest);

    let paused = host
        .expect(|m| match &m.message {
            Some(Reply::Paused(p)) => p.pause,
            _ => None,
        })
        .await;
    assert_eq!(paused.slot, 1);
    assert!(paused.budget_left_ms > 0);
    expect_error(
        host.request(place(&match_id, 0, 0)).await,
        proto::ErrorCode::CommandRejected,
    );

    let mut back = server.client().await;
    let welcome = back.hello_guest("Guest", &resume_token).await.unwrap();
    assert!(welcome.resumed);
    let catch_up = back
        .expect(|m| match &m.message {
            Some(Reply::CatchUp(c)) => Some(c.clone()),
            _ => None,
        })
        .await;
    assert_eq!(catch_up.header.unwrap().match_id, match_id);
    // Built before the returning seat counts as back: it still lists that seat's pause.
    assert_eq!(catch_up.pauses.len(), 1);
    assert_eq!(catch_up.pause, catch_up.pauses.first().copied());
    let resumed = host
        .expect(|m| match &m.message {
            Some(Reply::Resumed(r)) => Some(r.clone()),
            _ => None,
        })
        .await;
    assert!(resumed.countdown_ms > 0);
}

#[tokio::test]
async fn kicked_member_is_notified_and_can_keep_using_the_server() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let guest = server.guest("Guest").await;
    let guest_identity = guest.welcome.identity.clone();
    let mut guest = guest.client;
    let Reply::LobbyState(lobby) = host
        .request(Request::CreateLobby(proto::CreateLobby {
            name: String::new(),
            settings: Some(tictactoe_settings(3, 3)),
            max_seats: 0,
        }))
        .await
    else {
        panic!("create failed");
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
        host.request(Request::KickMember(proto::KickMember {
            member: guest_identity,
        }))
        .await,
    );
    let closed = guest
        .expect(|m| match &m.message {
            Some(Reply::LobbyClosed(c)) => Some(*c),
            _ => None,
        })
        .await;
    assert_eq!(closed.reason, proto::LobbyClosedReason::Kicked as i32);
    let Reply::LobbyList(_) = guest
        .request(Request::ListLobbies(proto::ListLobbies {}))
        .await
    else {
        panic!("kicked client must still be served");
    };
}

#[tokio::test]
async fn host_leaving_hands_the_lobby_over() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    let Reply::LobbyState(lobby) = host
        .request(Request::CreateLobby(proto::CreateLobby {
            name: String::new(),
            settings: Some(tictactoe_settings(3, 3)),
            max_seats: 0,
        }))
        .await
    else {
        panic!("create failed");
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
        host.request(Request::LeaveLobby(proto::LeaveLobby {}))
            .await,
    );
    let state = guest
        .expect(|m| {
            lobby_state(m).filter(|l| l.host.as_ref().is_some_and(|h| h.display_name == "Guest"))
        })
        .await;
    assert_eq!(state.seats.len(), 1);
}

#[tokio::test]
async fn chat_reaches_lobby_members() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    let _match_id = start_two_player_tictactoe(&mut host, &mut guest).await;
    expect_ok(
        host.request(Request::SendChat(proto::SendChat {
            scope: proto::ChatScope::Lobby as i32,
            text: " hi! ".into(),
        }))
        .await,
    );
    let chat = guest
        .expect(|m| match &m.message {
            Some(Reply::Chat(c)) => Some(c.clone()),
            _ => None,
        })
        .await;
    assert_eq!(chat.text, "hi!");
    assert_eq!(chat.sender.unwrap().display_name, "Host");
}

#[tokio::test]
async fn shared_replay_is_validated() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let Reply::LobbyState(_) = host
        .request(Request::CreateLobby(proto::CreateLobby {
            name: String::new(),
            settings: Some(tictactoe_settings(3, 3)),
            max_seats: 0,
        }))
        .await
    else {
        panic!("create failed");
    };
    let garbage = Request::ShareReplay(proto::ShareReplay {
        replay_file: vec![0xff, 0x01, 0x02],
        host_only_control: true,
    });
    expect_error(host.request(garbage).await, proto::ErrorCode::InvalidReplay);
    let huge_board = proto::ReplayFile {
        format_version: mg_protocol::REPLAY_FORMAT_VERSION,
        header: Some(proto::MatchHeader {
            settings: Some(tictactoe_settings(u32::MAX, 5)),
            ..Default::default()
        }),
        ..Default::default()
    };
    let request = Request::ShareReplay(proto::ShareReplay {
        replay_file: huge_board.encode_to_vec(),
        host_only_control: true,
    });
    expect_error(host.request(request).await, proto::ErrorCode::InvalidReplay);
}
