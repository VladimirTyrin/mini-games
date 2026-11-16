//! Hub scenarios driven through its command queue with in-memory connections, so timers run on
//! tokio's paused clock.

use std::sync::Arc;
use std::time::Duration;

use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as Request;
use mg_protocol::proto::server_message::Message as Reply;
use tokio::sync::{Semaphore, mpsc, oneshot};

use super::{Hub, HubCommand, HubHandle, SignIn};
use crate::auth::{AvatarUrl, DisplayName, PlayerIdKey};
use crate::authenticated::Authenticated;
use crate::ids::{ConnectionId, LobbyId, MatchId, ResumeToken, SessionId};
use crate::matches::MatchAborted;
use crate::outbound::{Outbound, OutboundReceiver};
use crate::timing::Timing;

fn start_hub() -> HubHandle {
    let (sender, receiver) = mpsc::unbounded_channel();
    let handle = HubHandle::new(sender);
    let hub = Hub::new(
        handle.clone(),
        Arc::new(Semaphore::new(1)),
        Timing::default(),
    );
    tokio::spawn(hub.run(receiver));
    handle
}

struct TestConnection {
    hub: HubHandle,
    id: ConnectionId,
    session: SessionId,
    receiver: OutboundReceiver,
    next_request: u32,
}

#[derive(Debug)]
enum SignInOutcome {
    Welcome(Box<proto::Welcome>),
    Refused(proto::ErrorResponse),
}

async fn next_message(receiver: &mut OutboundReceiver) -> proto::ServerMessage {
    let text = receiver.next().await.expect("connection queue closed");
    serde_json::from_str(text.as_str()).unwrap()
}

async fn sign_in(hub: &HubHandle, who: Authenticated) -> (Option<TestConnection>, SignInOutcome) {
    sign_in_resuming(hub, who, None).await
}

async fn sign_in_resuming(
    hub: &HubHandle,
    who: Authenticated,
    resume: Option<ResumeToken>,
) -> (Option<TestConnection>, SignInOutcome) {
    let id = ConnectionId::next();
    let (outbound, mut receiver) = Outbound::channel(id);
    let (reply, response) = oneshot::channel();
    hub.send(HubCommand::SignIn(SignIn {
        connection: id,
        outbound,
        who,
        session_token: String::new(),
        resume,
        server_info: proto::ServerInfo::default(),
        request_id: 1,
        reply,
    }));
    let session = response.await.unwrap();
    let outcome = match next_message(&mut receiver).await.message {
        Some(Reply::Welcome(welcome)) => SignInOutcome::Welcome(Box::new(welcome)),
        Some(Reply::Error(error)) => SignInOutcome::Refused(error),
        other => panic!("unexpected sign-in reply {other:?}"),
    };
    let connection = session.map(|session| TestConnection {
        hub: hub.clone(),
        id,
        session,
        receiver,
        next_request: 10,
    });
    (connection, outcome)
}

fn player_ids() -> PlayerIdKey {
    PlayerIdKey::new(b"hub-test-secret".to_vec())
}

fn google(subject: &str, name: &str) -> Authenticated {
    Authenticated::Google {
        account: player_ids().google(subject),
        name: DisplayName::parse(name).unwrap(),
        avatar: None,
    }
}

fn guest(name: &str) -> Authenticated {
    Authenticated::Guest {
        name: DisplayName::parse(name).unwrap(),
    }
}

fn shown_identity(outcome: &SignInOutcome) -> proto::Identity {
    match outcome {
        SignInOutcome::Welcome(welcome) => welcome.identity.clone().unwrap(),
        SignInOutcome::Refused(error) => panic!("refused: {error:?}"),
    }
}

impl TestConnection {
    fn send(&mut self, message: Request) -> u32 {
        self.next_request += 1;
        self.hub.send(HubCommand::Client {
            session: self.session.clone(),
            connection: self.id,
            request_id: self.next_request,
            message,
        });
        self.next_request
    }

    async fn expect<T>(&mut self, mut pick: impl FnMut(&proto::ServerMessage) -> Option<T>) -> T {
        loop {
            let message = next_message(&mut self.receiver).await;
            if let Some(found) = pick(&message) {
                return found;
            }
        }
    }

    async fn request(&mut self, message: Request) -> Reply {
        let request_id = self.send(message);
        self.expect(|m| {
            (m.request_id == request_id)
                .then(|| m.message.clone())
                .flatten()
        })
        .await
    }

    fn sign_out(self) {
        self.hub.send(HubCommand::SignOut {
            session: self.session,
            connection: self.id,
        });
    }
}

fn tictactoe() -> proto::GameSettings {
    <mg_tictactoe::TicTacToe as mg_protocol::WireGame>::settings_to_wire(
        &mg_tictactoe::RawSettings {
            field_width: 3,
            field_height: 3,
            win_count: 3,
            first_player: mg_tictactoe::FirstPlayerMode::Host,
        },
    )
}

async fn create_lobby(host: &mut TestConnection) -> proto::LobbyState {
    match host
        .request(Request::CreateLobby(proto::CreateLobby {
            name: "test".into(),
            settings: Some(tictactoe()),
            max_seats: 0,
        }))
        .await
    {
        Reply::LobbyState(state) => state,
        other => panic!("create failed: {other:?}"),
    }
}

#[tokio::test(start_paused = true)]
async fn everyone_keeps_their_name_and_players_are_told_apart_by_id() {
    let hub = start_hub();
    let (_ada, first) = sign_in(&hub, google("sub-1", "Ada")).await;
    let (other_ada, second) = sign_in(&hub, google("sub-2", "Ada")).await;
    let (_guest_ada, third) = sign_in(&hub, guest("Ada")).await;
    let (_bot_ada, fourth) = sign_in(
        &hub,
        Authenticated::BotClient {
            account: player_ids().bot_client(&DisplayName::parse("Ada").unwrap()),
            name: DisplayName::parse("Ada").unwrap(),
        },
    )
    .await;
    let identities: Vec<proto::Identity> = [&first, &second, &third, &fourth]
        .into_iter()
        .map(shown_identity)
        .collect();
    assert!(
        identities
            .iter()
            .all(|identity| identity.display_name == "Ada")
    );
    for (index, identity) in identities.iter().enumerate() {
        assert!(
            identities[index + 1..]
                .iter()
                .all(|other| other.id != identity.id)
        );
    }

    // The same account on another device (or after signing out) is another player.
    other_ada.unwrap().sign_out();
    let (_again, again) = sign_in(&hub, google("sub-2", "Ada")).await;
    assert_ne!(shown_identity(&again).id, identities[1].id);
}

#[tokio::test(start_paused = true)]
async fn an_account_back_within_the_grace_period_shows_its_new_name_and_avatar() {
    let hub = start_hub();
    let (host, signed_in) = sign_in(&hub, google("sub-1", "Ada")).await;
    let mut host = host.unwrap();
    let SignInOutcome::Welcome(first_welcome) = signed_in else {
        panic!("refused: {signed_in:?}");
    };
    let (member, _) = sign_in(&hub, guest("Member")).await;
    let mut member = member.unwrap();
    let lobby = create_lobby(&mut host).await;
    let joined = member
        .request(Request::JoinLobby(proto::JoinLobby {
            lobby_id: lobby.lobby_id.clone(),
            as_observer: false,
        }))
        .await;
    assert!(matches!(joined, Reply::LobbyState(_)));
    hub.send(HubCommand::Disconnected {
        session: host.session.clone(),
        connection: host.id,
    });

    let picture = "https://lh3.googleusercontent.com/a/new";
    let renamed = Authenticated::Google {
        account: player_ids().google("sub-1"),
        name: DisplayName::parse("Ada L").unwrap(),
        avatar: AvatarUrl::parse(picture),
    };
    let resume = ResumeToken::parse(&first_welcome.resume_token);
    let (again, outcome) = sign_in_resuming(&hub, renamed, resume).await;
    assert_eq!(again.unwrap().session, host.session);
    let SignInOutcome::Welcome(welcome) = outcome else {
        panic!("refused: {outcome:?}");
    };
    let identity = welcome.identity.unwrap();
    assert_eq!(identity.display_name, "Ada L");
    assert_eq!(identity.avatar_url, picture);

    let host_seen_by_member = member
        .expect(|m| match &m.message {
            Some(Reply::LobbyState(state)) => state
                .host
                .clone()
                .filter(|host| host.display_name == "Ada L"),
            _ => None,
        })
        .await;
    assert_eq!(host_seen_by_member.avatar_url, picture);
    assert_eq!(host_seen_by_member.id, identity.id);
}

#[tokio::test(start_paused = true)]
async fn a_sign_in_whose_connection_vanished_leaves_no_session_behind() {
    let hub = start_hub();
    let (outbound, mut vanished) = Outbound::channel(ConnectionId::next());
    let (reply, response) = oneshot::channel();
    drop(response);
    hub.send(HubCommand::SignIn(SignIn {
        connection: ConnectionId::next(),
        outbound,
        who: google("sub-1", "Ghost"),
        session_token: String::new(),
        resume: None,
        server_info: proto::ServerInfo::default(),
        request_id: 1,
        reply,
    }));
    let (connection, outcome) = sign_in(&hub, google("sub-1", "Ghost")).await;
    assert!(connection.is_some());
    assert_eq!(shown_identity(&outcome).display_name, "Ghost");
    // A leftover session would have been taken over, telling the vanished connection so.
    while let Some(text) = vanished.try_next() {
        let message: proto::ServerMessage = serde_json::from_str(text.as_str()).unwrap();
        assert!(
            !matches!(message.message, Some(Reply::Error(_))),
            "{message:?}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn ready_toggles_do_not_push_the_lobby_list() {
    let hub = start_hub();
    let (browser, _) = sign_in(&hub, guest("Browser")).await;
    let mut browser = browser.unwrap();
    let (host, _) = sign_in(&hub, guest("Host")).await;
    let mut host = host.unwrap();
    create_lobby(&mut host).await;
    let pushed = browser
        .expect(|m| match &m.message {
            Some(Reply::LobbyList(list)) => Some(list.clone()),
            _ => None,
        })
        .await;
    assert_eq!(pushed.lobbies.len(), 1);

    for ready in [true, false, true, false, true] {
        let reply = host
            .request(Request::SetReady(proto::SetReady { ready }))
            .await;
        assert!(matches!(reply, Reply::Ack(_)));
    }
    // Paused clock: this lets every coalescing window run out.
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(browser.receiver.try_next().is_none());
}

#[tokio::test(start_paused = true)]
async fn an_aborted_match_ends_without_result_and_reopens_the_lobby() {
    let hub = start_hub();
    let (host, _) = sign_in(&hub, guest("Host")).await;
    let mut host = host.unwrap();
    let (guest_connection, _) = sign_in(&hub, guest("Guest")).await;
    let mut guest_connection = guest_connection.unwrap();
    let lobby = create_lobby(&mut host).await;
    let joined = guest_connection
        .request(Request::JoinLobby(proto::JoinLobby {
            lobby_id: lobby.lobby_id.clone(),
            as_observer: false,
        }))
        .await;
    assert!(matches!(joined, Reply::LobbyState(_)));
    let ready = guest_connection
        .request(Request::SetReady(proto::SetReady { ready: true }))
        .await;
    assert!(matches!(ready, Reply::Ack(_)));
    let started = host
        .request(Request::StartMatch(proto::StartMatch {}))
        .await;
    assert!(matches!(started, Reply::Ack(_)));
    let header = host
        .expect(|m| match &m.message {
            Some(Reply::MatchStarted(header)) => Some(header.clone()),
            _ => None,
        })
        .await;

    hub.send(HubCommand::MatchAborted(MatchAborted {
        lobby: LobbyId::parse(&lobby.lobby_id).unwrap(),
        match_id: MatchId::parse(&header.match_id).unwrap(),
    }));
    for connection in [&mut host, &mut guest_connection] {
        let ended = connection
            .expect(|m| match &m.message {
                Some(Reply::Ended(ended)) => Some(ended.clone()),
                _ => None,
            })
            .await;
        assert_eq!(ended.match_id, header.match_id);
        assert!(ended.result.is_none());
        connection
            .expect(|m| match &m.message {
                Some(Reply::LobbyState(state)) if state.phase == proto::LobbyPhase::Open as i32 => {
                    Some(())
                }
                _ => None,
            })
            .await;
    }
}

#[tokio::test(start_paused = true)]
async fn a_device_back_without_its_resume_token_gets_its_seat_back() {
    let hub = start_hub();
    let (phone, _) = sign_in(&hub, google("sub-1", "Ada")).await;
    let mut phone = phone.unwrap();
    let lobby = create_lobby(&mut phone).await;
    hub.send(HubCommand::Disconnected {
        session: phone.session.clone(),
        connection: phone.id,
    });

    // The tab was closed (or the phone died): a fresh sign-in, no resume token.
    let (again, outcome) = sign_in(&hub, google("sub-1", "Ada")).await;

    assert_eq!(again.unwrap().session, phone.session);
    let SignInOutcome::Welcome(welcome) = outcome else {
        panic!("refused: {outcome:?}");
    };
    assert!(welcome.resumed, "back in lobby {}", lobby.lobby_id);
}

#[tokio::test(start_paused = true)]
async fn a_second_device_signing_in_while_the_first_is_connected_is_a_new_player() {
    let hub = start_hub();
    let (phone, _) = sign_in(&hub, google("sub-1", "Ada")).await;
    let (laptop, _) = sign_in(&hub, google("sub-1", "Ada")).await;

    assert_ne!(phone.unwrap().session, laptop.unwrap().session);
}
