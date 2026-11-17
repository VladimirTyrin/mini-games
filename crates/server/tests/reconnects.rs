//! Players coming back: resumed seats, devices taking over, matches that moved on meanwhile.

mod support;

use std::time::Duration;

use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as Request;
use mg_protocol::proto::server_message::Message as Reply;
use support::*;

/// Word Hunt with the host last: the bot (slot 1) has the first turn.
fn word_hunt_bot_first() -> proto::GameSettings {
    proto::GameSettings {
        game: Some(proto::game_settings::Game::WordHunt(
            proto::WordHuntSettings {
                language: proto::WordHuntLanguage::English as i32,
                word_length: 5,
                max_guesses: 6,
                hard_mode: false,
                mode: proto::WordHuntPuzzleMode::Unlimited as i32,
                day: 0,
                first_turn: proto::WordHuntFirstTurn::HostLast as i32,
            },
        )),
    }
}

fn bot_moved(message: &proto::ServerMessage) -> Option<()> {
    match &message.message {
        Some(Reply::Frame(frame))
            if frame.inputs.iter().any(|input| {
                matches!(&input.input, Some(proto::frame_input::Input::Command(command)) if command.slot == 1)
            }) =>
        {
            Some(())
        }
        _ => None,
    }
}

fn someone_lost(message: &proto::ServerMessage) -> Option<()> {
    lobby_state(message)?
        .seats
        .iter()
        .any(|seat| seat.connection_lost)
        .then_some(())
}

#[tokio::test]
async fn a_bot_on_turn_moves_as_soon_as_a_turn_based_match_resumes() {
    let server = TestServer::start_with(|config| config.timing = quick_timing()).await;
    let mut host = server.guest("Host").await.client;
    let guest = server.guest("Guest").await;
    let token = guest.welcome.resume_token.clone();
    let mut guest = guest.client;
    let lobby = create_lobby(&mut host, word_hunt_bot_first(), 3).await;
    expect_ok(
        host.request(Request::AddBot(proto::AddBot {
            bot_kind: Some(proto::BotKind {
                game: Some(proto::bot_kind::Game::WordHunt(
                    proto::WordHuntBotKind::Medium as i32,
                )),
            }),
        }))
        .await,
    );
    join_ready(&mut guest, &lobby.lobby_id).await;
    drop(guest);
    host.expect(someone_lost).await;
    start(&mut host).await;
    host.expect(paused).await;

    let mut back = server.client().await;
    back.hello_guest("Guest", &token).await.unwrap();

    host.expect(bot_moved).await;
}

#[tokio::test]
async fn a_device_that_was_away_cannot_take_its_session_back() {
    let server = TestServer::start_with(|config| {
        config.session_secret = Some(SESSION_SECRET.into());
    })
    .await;
    let mut phone = server.client().await;
    let phone_welcome = phone.hello(google_session("111", "Ada"), "").await.unwrap();
    let lobby = create_lobby(&mut phone, tictactoe_settings(3, 3), 2).await;
    let mut watcher = server.guest("Watcher").await.client;
    expect_ok(
        watcher
            .request(Request::JoinLobby(proto::JoinLobby {
                lobby_id: lobby.lobby_id,
                as_observer: true,
            }))
            .await,
    );
    drop(phone);
    watcher.expect(someone_lost).await;
    let mut laptop = server.client().await;
    let laptop_welcome = laptop
        .hello(google_session("111", "Ada"), "")
        .await
        .unwrap();
    assert!(
        laptop_welcome.resumed,
        "the laptop takes the away phone's seat"
    );

    let mut phone_again = server.client().await;
    let phone_back = phone_again
        .hello(google_session("111", "Ada"), &phone_welcome.resume_token)
        .await
        .unwrap();

    assert!(!phone_back.resumed, "the phone is a new player now");
    assert_ne!(phone_back.identity, laptop_welcome.identity);
    let lobby_list = laptop
        .request(Request::ListLobbies(proto::ListLobbies {}))
        .await;
    assert!(matches!(lobby_list, Reply::LobbyList(_)), "{lobby_list:?}");
}

#[tokio::test]
async fn a_player_back_after_the_match_ended_learns_the_result() {
    let server = TestServer::start_with(|config| {
        config.timing = quick_timing();
        config.timing.drop_vote_after = Duration::ZERO;
    })
    .await;
    let mut host = server.guest("Host").await.client;
    let guest = server.guest("Guest").await;
    let token = guest.welcome.resume_token.clone();
    let mut guest = guest.client;
    let match_id = start_two_player_tictactoe(&mut host, &mut guest).await;
    drop(guest);
    host.expect(paused).await;
    expect_ok(
        host.request(Request::VoteDropPlayer(proto::VoteDropPlayer {
            match_id: match_id.clone(),
            slot: 1,
        }))
        .await,
    );
    host.expect(match_ended).await;

    let mut back = server.client().await;
    let welcome = back.hello_guest("Guest", &token).await.unwrap();
    assert!(welcome.resumed);

    let ended = back.expect(match_ended).await;
    assert_eq!(ended.match_id, match_id);
    assert!(ended.result.is_some());
}
