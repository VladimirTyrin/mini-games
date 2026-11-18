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
use crate::social::SocialHandle;
use crate::timing::Timing;

fn start_hub() -> HubHandle {
    let (sender, receiver) = mpsc::unbounded_channel();
    let handle = HubHandle::new(sender);
    // Social commands go nowhere: these tests are about lobbies and sessions.
    let (social, _ignored) = mpsc::unbounded_channel();
    let hub = Hub::new(
        handle.clone(),
        SocialHandle::new(social),
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

fn start_hub_with(timing: Timing) -> HubHandle {
    let (sender, receiver) = mpsc::unbounded_channel();
    let handle = HubHandle::new(sender);
    let (social, _ignored) = mpsc::unbounded_channel();
    let hub = Hub::new(
        handle.clone(),
        SocialHandle::new(social),
        Arc::new(Semaphore::new(1)),
        timing,
    );
    tokio::spawn(hub.run(receiver));
    handle
}

#[tokio::test(start_paused = true)]
async fn a_seat_lost_in_the_lobby_is_kept_for_the_match_budget_not_the_lobby_grace() {
    let hub = start_hub_with(Timing {
        resume_grace: std::time::Duration::from_secs(10),
        disconnect_budget: std::time::Duration::from_secs(60),
        ..Timing::default()
    });
    let (host, _) = sign_in(&hub, guest("Host")).await;
    let mut host = host.unwrap();
    let (player, welcome) = sign_in(&hub, guest("Guest")).await;
    let mut player = player.unwrap();
    let SignInOutcome::Welcome(welcome) = welcome else {
        panic!("guest refused");
    };
    let lobby = create_lobby(&mut host).await;
    player
        .request(Request::JoinLobby(proto::JoinLobby {
            lobby_id: lobby.lobby_id,
            as_observer: false,
        }))
        .await;
    player
        .request(Request::SetReady(proto::SetReady { ready: true }))
        .await;
    hub.send(HubCommand::Disconnected {
        session: player.session.clone(),
        connection: player.id,
    });
    assert!(matches!(
        host.request(Request::StartMatch(proto::StartMatch {}))
            .await,
        Reply::Ack(_)
    ));

    // Past the lobby's grace, well within the match's budget. The woken grace timer posts to
    // the hub; a request answered after that shows the hub has handled it.
    tokio::time::advance(std::time::Duration::from_secs(25)).await;
    tokio::task::yield_now().await;
    host.request(Request::ListLobbies(proto::ListLobbies {}))
        .await;
    let (_back, outcome) = sign_in_resuming(
        &hub,
        guest("Guest"),
        ResumeToken::parse(&welcome.resume_token),
    )
    .await;

    let SignInOutcome::Welcome(back) = outcome else {
        panic!("refused");
    };
    assert!(back.resumed, "the seat in the running match was kept");
}

fn sea_battle() -> proto::GameSettings {
    <mg_sea_battle::SeaBattle as mg_protocol::WireGame>::settings_to_wire(
        &mg_sea_battle::RawSettings,
    )
}

fn classic_fleet() -> proto::GameSecretCommand {
    use mg_sea_battle::{Cell, Orientation, SeaBattle, SecretCommand, Ship};
    let ship = |x, y, length| {
        Ship::new(Cell::new(x, y).unwrap(), length, Orientation::Horizontal).unwrap()
    };
    let ships = vec![
        ship(0, 0, 4),
        ship(5, 0, 3),
        ship(0, 2, 3),
        ship(4, 2, 2),
        ship(7, 2, 2),
        ship(0, 4, 2),
        ship(3, 4, 1),
        ship(5, 4, 1),
        ship(7, 4, 1),
        ship(9, 4, 1),
    ];
    <SeaBattle as mg_protocol::WireGame>::secret_command_to_wire(&SecretCommand::PlaceFleet(ships))
}

/// Two guests in a running Sea Battle, and an observer.
struct SeaBattleTable {
    host: TestConnection,
    player: TestConnection,
    /// For the player to come back after a lost connection.
    player_resume: ResumeToken,
    observer: TestConnection,
    match_id: String,
}

async fn start_sea_battle(hub: &HubHandle) -> SeaBattleTable {
    let (host, _) = sign_in(hub, guest("Host")).await;
    let mut host = host.unwrap();
    let (player, welcome) = sign_in(hub, guest("Guest")).await;
    let mut player = player.unwrap();
    let SignInOutcome::Welcome(welcome) = welcome else {
        panic!("guest refused");
    };
    let (observer, _) = sign_in(hub, guest("Watcher")).await;
    let mut observer = observer.unwrap();
    let Reply::LobbyState(lobby) = host
        .request(Request::CreateLobby(proto::CreateLobby {
            name: "test".into(),
            settings: Some(sea_battle()),
            max_seats: 0,
        }))
        .await
    else {
        panic!("create failed");
    };
    player
        .request(Request::JoinLobby(proto::JoinLobby {
            lobby_id: lobby.lobby_id.clone(),
            as_observer: false,
        }))
        .await;
    player
        .request(Request::SetReady(proto::SetReady { ready: true }))
        .await;
    observer
        .request(Request::JoinLobby(proto::JoinLobby {
            lobby_id: lobby.lobby_id,
            as_observer: true,
        }))
        .await;
    assert!(matches!(
        host.request(Request::StartMatch(proto::StartMatch {}))
            .await,
        Reply::Ack(_)
    ));
    let started = |m: &proto::ServerMessage| match &m.message {
        Some(Reply::MatchStarted(header)) => Some(header.clone()),
        _ => None,
    };
    let header = host.expect(started).await;
    player.expect(started).await;
    observer.expect(started).await;
    assert_eq!(header.secret_setup_ms, 90_000, "people have time to place");
    SeaBattleTable {
        host,
        player,
        player_resume: ResumeToken::parse(&welcome.resume_token).unwrap(),
        observer,
        match_id: header.match_id,
    }
}

fn place_fleet(match_id: &str) -> Request {
    Request::SubmitSecretCommand(proto::SubmitSecretCommand {
        match_id: match_id.into(),
        command: Some(classic_fleet()),
    })
}

fn seat_secret(message: &proto::ServerMessage) -> Option<proto::GameSeatSecret> {
    match &message.message {
        Some(Reply::SeatSecret(secret)) => secret.secret.clone(),
        _ => None,
    }
}

fn has_event(
    message: &proto::ServerMessage,
    wanted: impl Fn(&proto::sea_battle_world_event::Event) -> bool,
) -> bool {
    let Some(Reply::Frame(frame)) = &message.message else {
        return false;
    };
    frame.inputs.iter().any(|input| match &input.input {
        Some(proto::frame_input::Input::World(event)) => match &event.game {
            Some(proto::game_world_event::Game::SeaBattle(event)) => {
                event.event.as_ref().is_some_and(&wanted)
            }
            _ => false,
        },
        _ => false,
    })
}

#[tokio::test(start_paused = true)]
async fn a_fleet_reaches_its_own_seat_and_spectators_while_the_other_seat_learns_it_is_placed() {
    let hub = start_hub();
    let SeaBattleTable {
        mut host,
        mut player,
        mut observer,
        match_id,
        ..
    } = start_sea_battle(&hub).await;

    let placed = host.request(place_fleet(&match_id)).await;
    assert!(matches!(placed, Reply::Ack(_)), "{placed:?}");
    let Some(proto::game_seat_secret::Game::SeaBattle(own)) = host.expect(seat_secret).await.game
    else {
        panic!("not a sea battle fleet");
    };
    assert_eq!(own.ships.len(), 10);

    player
        .expect(|message| {
            assert!(seat_secret(message).is_none(), "another seat's fleet was sent");
            has_event(message, |event| {
                matches!(event, proto::sea_battle_world_event::Event::FleetPlaced(placed) if placed.slot == 0)
            })
            .then_some(())
        })
        .await;
    // Spectators watch both fleets, told whose each is.
    let shown = observer
        .expect(|message| match &message.message {
            Some(proto::server_message::Message::SeatSecret(secret)) => Some(secret.clone()),
            _ => None,
        })
        .await;
    assert_eq!(shown.slot, Some(0));

    let again = host.request(place_fleet(&match_id)).await;
    assert!(
        matches!(&again, Reply::Error(error) if error.code == i32::from(proto::ErrorCode::CommandRejected)),
        "{again:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn people_out_of_time_get_a_random_fleet_and_the_battle_starts() {
    let hub = start_hub();
    let SeaBattleTable {
        mut host,
        mut player,
        ..
    } = start_sea_battle(&hub).await;

    tokio::time::advance(Duration::from_secs(91)).await;

    host.expect(seat_secret).await;
    player.expect(seat_secret).await;
    host.expect(|message| {
        has_event(message, |event| {
            matches!(
                event,
                proto::sea_battle_world_event::Event::FirstShooterChosen(_)
            )
        })
        .then_some(())
    })
    .await;
}

/// The time to place runs on while the match waits for someone who lost the connection: they
/// get a random fleet like anyone, and see it once back.
#[tokio::test(start_paused = true)]
async fn a_seat_away_when_time_runs_out_gets_a_random_fleet_and_sees_it_when_back() {
    let hub = start_hub_with(Timing {
        disconnect_budget: Duration::from_secs(300),
        ..Timing::default()
    });
    let SeaBattleTable {
        mut host,
        player,
        player_resume,
        match_id,
        ..
    } = start_sea_battle(&hub).await;
    hub.send(HubCommand::Disconnected {
        session: player.session.clone(),
        connection: player.id,
    });
    host.expect(|message| match &message.message {
        Some(Reply::Paused(_)) => Some(()),
        _ => None,
    })
    .await;
    // A fleet needs nobody else: it is taken while the match waits for the other seat.
    assert!(matches!(
        host.request(place_fleet(&match_id)).await,
        Reply::Ack(_)
    ));

    tokio::time::advance(Duration::from_secs(91)).await;
    host.expect(|message| {
        has_event(message, |event| {
            matches!(event, proto::sea_battle_world_event::Event::FleetPlaced(placed) if placed.slot == 1)
        })
        .then_some(())
    })
    .await;

    let (back, outcome) = sign_in_resuming(&hub, guest("Guest"), Some(player_resume)).await;
    assert!(matches!(outcome, SignInOutcome::Welcome(welcome) if welcome.resumed));
    let mut back = back.unwrap();
    let Some(proto::game_seat_secret::Game::SeaBattle(fleet)) = back.expect(seat_secret).await.game
    else {
        panic!("not a sea battle fleet");
    };
    assert_eq!(fleet.ships.len(), 10, "the random fleet placed while away");
}

/// A catch-up asked for (after a desync) brings the seat's own fleet again with the log.
#[tokio::test(start_paused = true)]
async fn a_catch_up_brings_the_own_fleet_again() {
    let hub = start_hub();
    let SeaBattleTable {
        mut host, match_id, ..
    } = start_sea_battle(&hub).await;
    assert!(matches!(
        host.request(place_fleet(&match_id)).await,
        Reply::Ack(_)
    ));
    host.expect(seat_secret).await;

    host.send(Request::RequestCatchUp(proto::RequestCatchUp { match_id }));
    host.expect(|message| match &message.message {
        Some(Reply::CatchUp(_)) => Some(()),
        _ => None,
    })
    .await;
    host.expect(seat_secret).await;
}
