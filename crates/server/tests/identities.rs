//! Players are told apart by `Identity.id`; display names are labels and may repeat.

mod support;

use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as Request;
use mg_protocol::proto::server_message::Message as Reply;
use support::*;

fn with_session_secret(config: &mut mg_server::Config) {
    config.session_secret = Some(SESSION_SECRET.into());
}

fn seated_ids(state: &proto::LobbyState) -> Vec<String> {
    state
        .seats
        .iter()
        .filter_map(|seat| seat.occupant.as_ref().map(|occupant| occupant.id.clone()))
        .collect()
}

#[tokio::test]
async fn two_guests_with_the_same_name_share_a_lobby_as_different_players() {
    let server = TestServer::start().await;
    let first = server.guest("Sam").await;
    let second = server.guest("Sam").await;
    let first_id = first.welcome.identity.clone().unwrap().id;
    let second_id = second.welcome.identity.clone().unwrap().id;
    assert_ne!(first_id, second_id);
    assert!(first_id.starts_with("u_") && second_id.starts_with("u_"));

    let (mut host, mut guest) = (first.client, second.client);
    let lobby = create_lobby(&mut host, tictactoe_settings(3, 3), 0).await;
    join_ready(&mut guest, &lobby.lobby_id).await;
    let state = host
        .expect(|m| lobby_state(m).filter(|state| state.seats.len() == 2))
        .await;
    assert_eq!(seated_ids(&state), [first_id.clone(), second_id.clone()]);
    assert!(state.seats.iter().all(|seat| {
        seat.occupant
            .as_ref()
            .is_some_and(|occupant| occupant.display_name == "Sam")
    }));
    assert_eq!(state.host.unwrap().id, first_id);

    let header = start(&mut host).await;
    let seat_ids: Vec<String> = header
        .seats
        .iter()
        .map(|seat| seat.player.clone().unwrap().id)
        .collect();
    assert_eq!(seat_ids, [first_id, second_id]);
}

#[tokio::test]
async fn kicking_by_id_removes_only_that_one_of_two_namesakes() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let staying = server.guest("Sam").await;
    let leaving = server.guest("Sam").await;
    let staying_id = staying.welcome.identity.clone().unwrap().id;
    let leaving_id = leaving.welcome.identity.clone().unwrap().id;
    let (mut staying, mut leaving) = (staying.client, leaving.client);
    let lobby = create_lobby(&mut host, tictactoe_settings(3, 3), 0).await;
    expect_ok(
        staying
            .request(Request::JoinLobby(proto::JoinLobby {
                lobby_id: lobby.lobby_id.clone(),
                as_observer: true,
            }))
            .await,
    );
    join_ready(&mut leaving, &lobby.lobby_id).await;
    staying
        .expect(|m| lobby_state(m).filter(|state| state.seats.len() == 2))
        .await;

    expect_ok(
        host.request(Request::KickMember(proto::KickMember {
            player_id: leaving_id.clone(),
        }))
        .await,
    );
    leaving
        .expect(|m| match &m.message {
            Some(Reply::LobbyClosed(closed))
                if closed.reason == proto::LobbyClosedReason::Kicked as i32 =>
            {
                Some(())
            }
            _ => None,
        })
        .await;
    let state = staying
        .expect(|m| lobby_state(m).filter(|state| state.seats.len() == 1))
        .await;
    assert!(!seated_ids(&state).contains(&leaving_id));
    let observer_ids: Vec<&str> = state.observers.iter().map(|o| o.id.as_str()).collect();
    assert_eq!(observer_ids, [staying_id.as_str()]);
}

#[tokio::test]
async fn the_lobby_list_names_each_host_by_identity() {
    let server = TestServer::start().await;
    let host = server.guest("Sam").await;
    let host_id = host.welcome.identity.clone().unwrap().id;
    let mut host = host.client;
    create_lobby(&mut host, tictactoe_settings(3, 3), 0).await;
    let mut browser = server.guest("Sam").await.client;
    let Reply::LobbyList(list) = browser
        .request(Request::ListLobbies(proto::ListLobbies {}))
        .await
    else {
        panic!("lobby list expected");
    };
    assert_eq!(list.lobbies[0].host.as_ref().unwrap().id, host_id);
}

#[tokio::test]
async fn a_google_account_keeps_an_opaque_id_on_every_connection() {
    const SUBJECT: &str = "109876543210987654321";
    let server = TestServer::start_with(with_session_secret).await;
    let mut first = server.client().await;
    let first_identity = first
        .hello(google_session(SUBJECT, "Ada"), "")
        .await
        .unwrap()
        .identity
        .unwrap();
    assert_eq!(first_identity.kind, proto::IdentityKind::Google as i32);
    assert!(first_identity.id.starts_with("g_"), "{}", first_identity.id);
    assert!(!first_identity.id.contains(SUBJECT));

    // Another device, with a changed profile name: same player, and it takes the session over.
    let mut second = server.client().await;
    let second_identity = second
        .hello(google_session(SUBJECT, "Ada Lovelace"), "")
        .await
        .unwrap()
        .identity
        .unwrap();
    assert_eq!(second_identity.id, first_identity.id);
    assert_eq!(second_identity.display_name, "Ada Lovelace");
    let error = first.expect(error_push).await;
    assert_eq!(error.code, proto::ErrorCode::SignedInElsewhere as i32);
    assert!(first.closed().await);

    // After a restart with the same secret, still the same id; another account gets another.
    let restarted = TestServer::start_with(with_session_secret).await;
    let mut later = restarted.client().await;
    let later_identity = later
        .hello(google_session(SUBJECT, "Ada"), "")
        .await
        .unwrap()
        .identity
        .unwrap();
    assert_eq!(later_identity.id, first_identity.id);
    let mut other = restarted.client().await;
    let other_identity = other
        .hello(google_session("123", "Ada"), "")
        .await
        .unwrap()
        .identity
        .unwrap();
    assert_ne!(other_identity.id, first_identity.id);
}

#[tokio::test]
async fn a_bot_gets_in_next_to_a_guest_of_the_same_name_with_a_stable_id() {
    let configure = |config: &mut mg_server::Config| {
        config.bot_keys = ALPHA_BOT_KEY.into();
        with_session_secret(config);
    };
    let server = TestServer::start_with(configure).await;
    let guest = server.guest("alpha").await;
    let mut bot = server.client().await;
    let bot_identity = bot
        .hello(bot_credentials("secret"), "")
        .await
        .unwrap()
        .identity
        .unwrap();
    assert_eq!(bot_identity.display_name, "alpha");
    assert!(bot_identity.id.starts_with("b_"), "{}", bot_identity.id);
    assert_ne!(bot_identity.id, guest.welcome.identity.unwrap().id);

    let restarted = TestServer::start_with(configure).await;
    let mut again = restarted.client().await;
    let again_identity = again
        .hello(bot_credentials("secret"), "")
        .await
        .unwrap()
        .identity
        .unwrap();
    assert_eq!(again_identity.id, bot_identity.id);
}
