//! Hub scenarios driven through its command queue with in-memory connections, so timers run on
//! tokio's paused clock.

use std::sync::Arc;
use std::time::Duration;

use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as Request;
use mg_protocol::proto::server_message::Message as Reply;
use tokio::sync::{Semaphore, mpsc, oneshot};

use super::{Hub, HubCommand, HubHandle, SignIn};
use crate::auth::{AvatarUrl, DisplayName};
use crate::authenticated::Authenticated;
use crate::ids::{ConnectionId, LobbyId, MatchId, SessionId};
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
    let id = ConnectionId::next();
    let (outbound, mut receiver) = Outbound::channel(id);
    let (reply, response) = oneshot::channel();
    hub.send(HubCommand::SignIn(SignIn {
        connection: id,
        outbound,
        who,
        session_token: String::new(),
        resume: None,
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

fn google(subject: &str, name: &str) -> Authenticated {
    Authenticated::Google {
        subject: subject.into(),
        name: DisplayName::parse(name).unwrap(),
        avatar: None,
    }
}

fn guest(name: &str) -> Authenticated {
    Authenticated::Guest {
        name: DisplayName::parse(name).unwrap(),
    }
}

fn shown_name(outcome: &SignInOutcome) -> String {
    match outcome {
        SignInOutcome::Welcome(welcome) => welcome.identity.clone().unwrap().display_name,
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
    <mg_tictactoe::TicTacToe as mg_protocol::WireGame>::settings_to_wire(&mg_tictactoe::Settings {
        field_width: 3,
        field_height: 3,
        win_count: 3,
        first_player: mg_tictactoe::FirstPlayerMode::Host,
    })
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
async fn accounts_with_a_taken_name_get_a_stable_suffix_and_guests_are_refused() {
    let hub = start_hub();
    let (_ada, first) = sign_in(&hub, google("sub-1", "Ada")).await;
    assert_eq!(shown_name(&first), "Ada");

    let (second_ada, second) = sign_in(&hub, google("sub-2", "ada")).await;
    let suffixed = shown_name(&second);
    assert!(suffixed.starts_with("ada#"), "{suffixed}");

    // Same account again later: same suffix.
    second_ada.unwrap().sign_out();
    let (_again, again) = sign_in(&hub, google("sub-2", "ada")).await;
    assert_eq!(shown_name(&again), suffixed);

    // A guest cannot take any name in use, suffixed or not.
    for taken in ["ADA", suffixed.as_str()] {
        let (refused, outcome) = sign_in(&hub, guest(taken)).await;
        assert!(refused.is_none());
        assert!(matches!(
            outcome,
            SignInOutcome::Refused(error) if error.code == proto::ErrorCode::NameTaken as i32
        ));
    }

    // A guest's name does not keep a bot account out either.
    let (_guest, _) = sign_in(&hub, guest("Alpha")).await;
    let (_bot, bot) = sign_in(
        &hub,
        Authenticated::BotClient {
            name: DisplayName::parse("alpha").unwrap(),
        },
    )
    .await;
    assert!(shown_name(&bot).starts_with("alpha#"));
}

#[tokio::test(start_paused = true)]
async fn an_account_back_within_the_grace_period_shows_its_new_name_and_avatar() {
    let hub = start_hub();
    let (host, _) = sign_in(&hub, google("sub-1", "Ada")).await;
    let mut host = host.unwrap();
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
        subject: "sub-1".into(),
        name: DisplayName::parse("Ada L").unwrap(),
        avatar: AvatarUrl::parse(picture),
    };
    let (again, outcome) = sign_in(&hub, renamed).await;
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

    // The old name is free again.
    let (guest_ada, _) = sign_in(&hub, guest("Ada")).await;
    assert!(guest_ada.is_some());
}

#[tokio::test(start_paused = true)]
async fn a_sign_in_whose_connection_vanished_does_not_keep_the_name() {
    let hub = start_hub();
    let (outbound, _receiver) = Outbound::channel(ConnectionId::next());
    let (reply, response) = oneshot::channel();
    drop(response);
    hub.send(HubCommand::SignIn(SignIn {
        connection: ConnectionId::next(),
        outbound,
        who: guest("Ghost"),
        session_token: String::new(),
        resume: None,
        server_info: proto::ServerInfo::default(),
        request_id: 1,
        reply,
    }));
    let (connection, outcome) = sign_in(&hub, guest("Ghost")).await;
    assert!(connection.is_some());
    assert_eq!(shown_name(&outcome), "Ghost");
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
