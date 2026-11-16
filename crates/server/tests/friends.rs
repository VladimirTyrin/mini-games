mod support;

use std::time::Duration;

use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as Request;
use mg_protocol::proto::server_message::Message as Reply;
use support::*;

async fn server_with(configure: impl FnOnce(&mut mg_server::Config)) -> TestServer {
    TestServer::start_with(|config| {
        config.session_secret = Some(SESSION_SECRET.into());
        configure(config);
    })
    .await
}

/// A connected device of a Google account and the id of that account.
async fn device(server: &TestServer, subject: &str, name: &str) -> (Client, String) {
    let mut client = server.client().await;
    let identity = client
        .hello(google_session(subject, name), "")
        .await
        .unwrap()
        .identity
        .unwrap();
    let account = identity.id.split('.').next().unwrap().to_owned();
    (client, account)
}

async fn friend_list(client: &mut Client) -> proto::FriendList {
    match client
        .request(Request::ListFriends(proto::ListFriends {}))
        .await
    {
        Reply::FriendList(list) => list,
        other => panic!("expected a friend list, got {other:?}"),
    }
}

fn friend_names(list: &proto::FriendList) -> Vec<String> {
    list.friends
        .iter()
        .map(|friend| friend.identity.as_ref().unwrap().display_name.clone())
        .collect()
}

async fn friend_link(client: &mut Client, kind: proto::FriendLinkKind) -> String {
    match client
        .request(Request::CreateFriendLink(proto::CreateFriendLink {
            kind: kind as i32,
        }))
        .await
    {
        Reply::FriendLink(link) => link.code,
        other => panic!("expected a friend link, got {other:?}"),
    }
}

async fn use_link(client: &mut Client, code: &str) -> Reply {
    client
        .request(Request::UseFriendLink(proto::UseFriendLink {
            code: code.to_owned(),
        }))
        .await
}

async fn befriend(a: &mut Client, b: &mut Client) {
    let code = friend_link(a, proto::FriendLinkKind::SingleUse).await;
    expect_ok(use_link(b, &code).await);
}

fn game_invite(message: &proto::ServerMessage) -> Option<proto::GameInvite> {
    match &message.message {
        Some(Reply::GameInvite(invite)) => Some(invite.clone()),
        _ => None,
    }
}

fn invite_state(state: proto::InviteState) -> impl FnMut(&proto::ServerMessage) -> Option<()> {
    move |message| match &message.message {
        Some(Reply::InviteUpdate(update)) if update.state == state as i32 => Some(()),
        _ => None,
    }
}

fn answer(invite_id: &str, answer: proto::InviteAnswer) -> Request {
    Request::AnswerInvite(proto::AnswerInvite {
        invite_id: invite_id.to_owned(),
        answer: answer as i32,
    })
}

#[tokio::test]
async fn a_friend_link_makes_friends_who_see_each_other_online() {
    let server = server_with(|_| {}).await;
    let (mut ada, _) = device(&server, "1", "Ada").await;
    let (mut bob, _) = device(&server, "2", "Bob").await;

    befriend(&mut ada, &mut bob).await;

    let adas = friend_list(&mut ada).await;
    assert_eq!(friend_names(&adas), ["Bob"]);
    assert_eq!(adas.friends[0].presence, proto::Presence::Online as i32);
    assert_eq!(friend_names(&friend_list(&mut bob).await), ["Ada"]);
}

#[tokio::test]
async fn a_single_use_link_works_once_and_a_chat_link_for_everyone() {
    let server = server_with(|_| {}).await;
    let (mut ada, _) = device(&server, "1", "Ada").await;
    let (mut bob, _) = device(&server, "2", "Bob").await;
    let (mut cid, _) = device(&server, "3", "Cid").await;

    let single = friend_link(&mut ada, proto::FriendLinkKind::SingleUse).await;
    expect_ok(use_link(&mut bob, &single).await);
    expect_error(
        use_link(&mut cid, &single).await,
        proto::ErrorCode::FriendLinkInvalid,
    );
    let chat = friend_link(&mut ada, proto::FriendLinkKind::MultiUse).await;
    expect_ok(use_link(&mut cid, &chat).await);

    assert_eq!(friend_names(&friend_list(&mut ada).await), ["Bob", "Cid"]);
}

#[tokio::test]
async fn a_lobby_member_asked_to_be_friends_can_accept() {
    let server = server_with(|_| {}).await;
    let (mut ada, ada_account) = device(&server, "1", "Ada").await;
    let (mut bob, _) = device(&server, "2", "Bob").await;
    let lobby = create_lobby(&mut ada, tictactoe_settings(3, 3), 2).await;
    join_ready(&mut bob, &lobby.lobby_id).await;
    let bob_device = ada
        .expect(|message| {
            lobby_state(message)
                .and_then(|state| state.seats.get(1).cloned())
                .and_then(|seat| seat.occupant)
        })
        .await
        .id;

    expect_ok(
        ada.request(Request::SendFriendRequest(proto::SendFriendRequest {
            player_id: bob_device,
        }))
        .await,
    );
    let incoming = friend_list(&mut bob).await.incoming_requests;
    expect_ok(
        bob.request(Request::AnswerFriendRequest(proto::AnswerFriendRequest {
            account_id: ada_account,
            answer: proto::FriendRequestAnswer::Accept as i32,
        }))
        .await,
    );

    assert_eq!(incoming[0].display_name, "Ada");
    assert_eq!(friend_names(&friend_list(&mut ada).await), ["Bob"]);
}

#[tokio::test]
async fn guests_cannot_have_friends() {
    let server = server_with(|_| {}).await;
    let mut guest = server.guest("Gus").await.client;

    let reply = guest
        .request(Request::ListFriends(proto::ListFriends {}))
        .await;

    expect_error(reply, proto::ErrorCode::AccountRequired);
}

#[tokio::test]
async fn an_invitation_reaches_every_device_and_one_answer_closes_it_everywhere() {
    let server = server_with(|_| {}).await;
    let (mut ada, _) = device(&server, "1", "Ada").await;
    let (mut phone, bob_account) = device(&server, "2", "Bob").await;
    let (mut tablet, _) = device(&server, "2", "Bob").await;
    befriend(&mut ada, &mut phone).await;
    let lobby = create_lobby(&mut ada, tictactoe_settings(3, 3), 2).await;

    ada.send(Request::InviteFriend(proto::InviteFriend {
        account_id: bob_account,
    }))
    .await;
    ada.expect(invite_state(proto::InviteState::Pending)).await;
    let on_phone = phone.expect(game_invite).await;
    let on_tablet = tablet.expect(game_invite).await;
    expect_ok(
        phone
            .request(answer(&on_phone.invite_id, proto::InviteAnswer::Accept))
            .await,
    );

    assert_eq!(on_phone.invite_id, on_tablet.invite_id);
    assert_eq!(on_phone.lobby_id, lobby.lobby_id);
    assert_eq!(on_phone.game, "tictactoe");
    ada.expect(invite_state(proto::InviteState::Accepted)).await;
    let closed = tablet
        .expect(|message| match &message.message {
            Some(Reply::GameInviteClosed(closed)) => Some(closed.clone()),
            _ => None,
        })
        .await;
    assert_eq!(closed.state, proto::InviteState::Accepted as i32);
    join_ready(&mut phone, &on_phone.lobby_id).await;
}

#[tokio::test]
async fn an_unanswered_invitation_expires() {
    let server = server_with(|config| {
        config.timing.invite_lifetime = Duration::from_millis(50);
    })
    .await;
    let (mut ada, _) = device(&server, "1", "Ada").await;
    let (mut bob, bob_account) = device(&server, "2", "Bob").await;
    befriend(&mut ada, &mut bob).await;
    create_lobby(&mut ada, tictactoe_settings(3, 3), 2).await;

    ada.send(Request::InviteFriend(proto::InviteFriend {
        account_id: bob_account,
    }))
    .await;
    let invite = bob.expect(game_invite).await;
    ada.expect(invite_state(proto::InviteState::Expired)).await;

    expect_error(
        bob.request(answer(&invite.invite_id, proto::InviteAnswer::Accept))
            .await,
        proto::ErrorCode::InviteClosed,
    );
}

#[tokio::test]
async fn only_friends_can_be_invited() {
    let server = server_with(|_| {}).await;
    let (mut ada, _) = device(&server, "1", "Ada").await;
    let (_bob, bob_account) = device(&server, "2", "Bob").await;
    create_lobby(&mut ada, tictactoe_settings(3, 3), 2).await;

    let reply = ada
        .request(Request::InviteFriend(proto::InviteFriend {
            account_id: bob_account,
        }))
        .await;

    expect_error(reply, proto::ErrorCode::NotFriends);
}

#[tokio::test]
async fn friends_see_each_other_go_offline() {
    let server = server_with(|_| {}).await;
    let (mut ada, _) = device(&server, "1", "Ada").await;
    let (mut bob, bob_account) = device(&server, "2", "Bob").await;
    befriend(&mut ada, &mut bob).await;

    drop(bob);

    ada.expect(|message| match &message.message {
        Some(Reply::FriendPresence(update))
            if update.account_id == bob_account
                && update.presence == proto::Presence::Offline as i32 =>
        {
            Some(())
        }
        _ => None,
    })
    .await;
}

async fn invite(inviter: &mut Client, account_id: &str) {
    inviter
        .send(Request::InviteFriend(proto::InviteFriend {
            account_id: account_id.to_owned(),
        }))
        .await;
    inviter
        .expect(invite_state(proto::InviteState::Pending))
        .await;
}

fn invite_closed(message: &proto::ServerMessage) -> Option<proto::GameInviteClosed> {
    match &message.message {
        Some(Reply::GameInviteClosed(closed)) => Some(closed.clone()),
        _ => None,
    }
}

#[tokio::test]
async fn the_inviter_hears_the_answer_after_reconnecting() {
    let server = server_with(|_| {}).await;
    let mut ada = server.client().await;
    let welcome = ada.hello(google_session("1", "Ada"), "").await.unwrap();
    let (mut bob, bob_account) = device(&server, "2", "Bob").await;
    befriend(&mut ada, &mut bob).await;
    create_lobby(&mut ada, tictactoe_settings(3, 3), 2).await;
    invite(&mut ada, &bob_account).await;
    let invitation = bob.expect(game_invite).await;

    drop(ada);
    let mut ada = server.client().await;
    ada.hello(google_session("1", "Ada"), &welcome.resume_token)
        .await
        .unwrap();
    expect_ok(
        bob.request(answer(&invitation.invite_id, proto::InviteAnswer::Decline))
            .await,
    );

    ada.expect(invite_state(proto::InviteState::Declined)).await;
}

#[tokio::test]
async fn leaving_the_lobby_closes_its_invitations() {
    let server = server_with(|_| {}).await;
    let (mut ada, _) = device(&server, "1", "Ada").await;
    let (mut bob, bob_account) = device(&server, "2", "Bob").await;
    befriend(&mut ada, &mut bob).await;
    create_lobby(&mut ada, tictactoe_settings(3, 3), 2).await;
    invite(&mut ada, &bob_account).await;
    let first = bob.expect(game_invite).await;

    expect_ok(ada.request(Request::LeaveLobby(proto::LeaveLobby {})).await);
    let closed = bob.expect(invite_closed).await;
    create_lobby(&mut ada, tictactoe_settings(3, 3), 2).await;
    invite(&mut ada, &bob_account).await;
    let second = bob.expect(game_invite).await;

    assert_eq!(closed.invite_id, first.invite_id);
    assert_eq!(closed.state, proto::InviteState::Cancelled as i32);
    assert_ne!(second.invite_id, first.invite_id);
    expect_error(
        bob.request(answer(&first.invite_id, proto::InviteAnswer::Accept))
            .await,
        proto::ErrorCode::InviteClosed,
    );
}

#[tokio::test]
async fn removing_a_friend_closes_the_invitations_between_them() {
    let server = server_with(|_| {}).await;
    let (mut ada, ada_account) = device(&server, "1", "Ada").await;
    let (mut bob, bob_account) = device(&server, "2", "Bob").await;
    befriend(&mut ada, &mut bob).await;
    create_lobby(&mut ada, tictactoe_settings(3, 3), 2).await;
    invite(&mut ada, &bob_account).await;
    let invitation = bob.expect(game_invite).await;

    expect_ok(
        bob.request(Request::RemoveFriend(proto::RemoveFriend {
            account_id: ada_account,
        }))
        .await,
    );

    ada.expect(invite_state(proto::InviteState::Cancelled))
        .await;
    expect_error(
        bob.request(answer(&invitation.invite_id, proto::InviteAnswer::Accept))
            .await,
        proto::ErrorCode::InviteClosed,
    );
}

#[tokio::test]
async fn a_device_that_signs_in_during_an_invitation_still_gets_it() {
    let server = server_with(|_| {}).await;
    let (mut ada, _) = device(&server, "1", "Ada").await;
    let (mut phone, bob_account) = device(&server, "2", "Bob").await;
    befriend(&mut ada, &mut phone).await;
    create_lobby(&mut ada, tictactoe_settings(3, 3), 2).await;
    invite(&mut ada, &bob_account).await;
    let on_phone = phone.expect(game_invite).await;

    let (mut tablet, _) = device(&server, "2", "Bob").await;

    let on_tablet = tablet.expect(game_invite).await;
    assert_eq!(on_tablet.invite_id, on_phone.invite_id);
    assert!(on_tablet.expires_in_ms > 0);
}
