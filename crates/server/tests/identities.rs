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
async fn each_device_of_a_google_account_is_its_own_player() {
    const SUBJECT: &str = "109876543210987654321";
    let server = TestServer::start_with(with_session_secret).await;
    let mut phone = server.client().await;
    let phone_identity = phone
        .hello(google_session(SUBJECT, "Ada"), "")
        .await
        .unwrap()
        .identity
        .unwrap();
    assert_eq!(phone_identity.kind, proto::IdentityKind::Google as i32);
    assert!(phone_identity.id.starts_with("g_"), "{}", phone_identity.id);
    assert!(!phone_identity.id.contains(SUBJECT));

    // A second device signs in too: nobody is pushed out, and the two are told apart.
    let mut laptop = server.client().await;
    let laptop_identity = laptop
        .hello(google_session(SUBJECT, "Ada"), "")
        .await
        .unwrap()
        .identity
        .unwrap();
    assert_ne!(laptop_identity.id, phone_identity.id);

    let lobby = create_lobby(&mut phone, tictactoe_settings(3, 3), 2).await;
    join_ready(&mut laptop, &lobby.lobby_id).await;
    let state = phone
        .expect(|m| lobby_state(m).filter(|state| state.seats.len() == 2))
        .await;
    assert_eq!(
        seated_ids(&state),
        vec![phone_identity.id.clone(), laptop_identity.id.clone()]
    );
    // Still connected: the phone's requests are answered.
    let Reply::LobbyList(_) = phone
        .request(Request::ListLobbies(proto::ListLobbies {}))
        .await
    else {
        panic!("the first device must stay signed in");
    };
}

#[tokio::test]
async fn a_device_resumes_its_own_session_even_after_a_profile_change() {
    const SUBJECT: &str = "109876543210987654321";
    let server = TestServer::start_with(with_session_secret).await;
    let mut first = server.client().await;
    let welcome = first
        .hello(google_session(SUBJECT, "Ada"), "")
        .await
        .unwrap();
    let first_id = welcome.identity.unwrap().id;
    drop(first);

    let mut again = server.client().await;
    let resumed = again
        .hello(
            google_session(SUBJECT, "Ada Lovelace"),
            &welcome.resume_token,
        )
        .await
        .unwrap();
    let identity = resumed.identity.unwrap();
    assert_eq!(identity.id, first_id);
    assert_eq!(identity.display_name, "Ada Lovelace");
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
