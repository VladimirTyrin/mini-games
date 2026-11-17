//! Lobby and match transitions: pauses, drops, budgets, play-again, shared replays.

mod support;

use std::time::Duration;

use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as Request;
use mg_protocol::proto::server_message::Message as Reply;
use support::*;

fn seat_lost(name: &'static str, lost: bool) -> impl FnMut(&proto::ServerMessage) -> Option<()> {
    move |message| {
        let state = lobby_state(message)?;
        state
            .seats
            .iter()
            .any(|seat| {
                seat.occupant
                    .as_ref()
                    .is_some_and(|o| o.display_name == name)
                    && seat.connection_lost == lost
            })
            .then_some(())
    }
}

#[tokio::test]
async fn a_voted_out_seat_cannot_pause_the_match_again_or_play() {
    let server = TestServer::start_with(|config| {
        config.timing = quick_timing();
        config.timing.drop_vote_after = Duration::ZERO;
        config.timing.resume_countdown = Duration::ZERO;
    })
    .await;
    let mut host = server.guest("Host").await.client;
    let mut stays = server.guest("Stays").await.client;
    let leaver = server.guest("Leaver").await;
    let resume_token = leaver.welcome.resume_token.clone();
    let mut leaver = leaver.client;
    let lobby = create_lobby(&mut host, snake_settings(), 3).await;
    join_ready(&mut stays, &lobby.lobby_id).await;
    join_ready(&mut leaver, &lobby.lobby_id).await;
    let match_id = start(&mut host).await.match_id;
    drop(leaver);

    let pause = host.expect(paused).await;
    assert_eq!((pause.slot, pause.drop_vote_in_ms), (2, 0));
    expect_ok(
        host.request(Request::VoteDropPlayer(proto::VoteDropPlayer {
            match_id: match_id.clone(),
            slot: 2,
        }))
        .await,
    );
    host.expect(drops_slot(2)).await;

    let mut back = server.client().await;
    assert!(
        back.hello_guest("Leaver", &resume_token)
            .await
            .unwrap()
            .resumed
    );
    back.expect(catch_up).await;
    expect_error(
        back.request(turn_left(&match_id)).await,
        proto::ErrorCode::CommandRejected,
    );
    host.expect(seat_lost("Leaver", false)).await;
    drop(back);
    host.expect(seat_lost("Leaver", true)).await;

    // Not paused: the dropped seat's second disconnect must not freeze the match.
    expect_ok(host.request(turn_left(&match_id)).await);
}

fn paused_seats(message: &proto::ServerMessage) -> Option<Vec<u32>> {
    match &message.message {
        Some(Reply::Paused(paused)) => Some(paused.pauses.iter().map(|p| p.slot).collect()),
        _ => None,
    }
}

#[tokio::test]
async fn a_pause_lists_only_the_seats_still_away_after_one_returns() {
    let server = TestServer::start_with(|config| config.timing = quick_timing()).await;
    let mut host = server.guest("Host").await.client;
    let first = server.guest("First").await;
    let first_resume = first.welcome.resume_token.clone();
    let mut first = first.client;
    let mut second = server.guest("Second").await.client;
    let lobby = create_lobby(&mut host, snake_settings(), 3).await;
    join_ready(&mut first, &lobby.lobby_id).await;
    join_ready(&mut second, &lobby.lobby_id).await;
    start(&mut host).await;

    drop(first);
    assert_eq!(host.expect(paused_seats).await, vec![1]);
    drop(second);
    assert_eq!(host.expect(paused_seats).await, vec![1, 2]);

    let mut back = server.client().await;
    assert!(
        back.hello_guest("First", &first_resume)
            .await
            .unwrap()
            .resumed
    );
    assert_eq!(host.expect(paused_seats).await, vec![2]);
}

#[tokio::test]
async fn an_exhausted_disconnect_budget_drops_the_seat() {
    let server = TestServer::start_with(|config| {
        config.timing = quick_timing();
        config.timing.disconnect_budget = Duration::from_millis(100);
    })
    .await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    start_two_player_tictactoe(&mut host, &mut guest).await;
    drop(guest);
    let pause = host.expect(paused).await;
    assert!(pause.budget_left_ms <= 100);
    host.expect(drops_slot(1)).await;
}

#[tokio::test]
async fn a_lobby_seat_is_freed_when_the_resume_grace_runs_out() {
    let server = TestServer::start_with(|config| {
        config.timing = quick_timing();
        config.timing.resume_grace = Duration::from_millis(100);
    })
    .await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    let lobby = create_lobby(&mut host, tictactoe_settings(3, 3), 0).await;
    join_ready(&mut guest, &lobby.lobby_id).await;
    drop(guest);
    host.expect(seat_lost("Guest", true)).await;
    host.expect(|m| lobby_state(m).filter(|l| l.seats.len() == 1))
        .await;
    // The name is free again.
    let _again = server.guest("Guest").await;
}

#[tokio::test]
async fn a_seat_disconnected_before_the_start_pauses_the_new_match() {
    let server = TestServer::start_with(|config| config.timing = quick_timing()).await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    let lobby = create_lobby(&mut host, tictactoe_settings(3, 3), 0).await;
    join_ready(&mut guest, &lobby.lobby_id).await;
    drop(guest);
    host.expect(seat_lost("Guest", true)).await;
    let match_id = start(&mut host).await.match_id;
    assert_eq!(host.expect(paused).await.slot, 1);
    expect_error(
        host.request(place(&match_id, 0, 0)).await,
        proto::ErrorCode::CommandRejected,
    );
}

#[tokio::test]
async fn an_observer_joining_mid_match_gets_the_history_and_catch_ups_are_rate_limited() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    let match_id = start_two_player_tictactoe(&mut host, &mut guest).await;
    expect_ok(host.request(place(&match_id, 1, 1)).await);

    let Reply::LobbyList(list) = host
        .request(Request::ListLobbies(proto::ListLobbies {}))
        .await
    else {
        panic!("expected the lobby list");
    };
    let mut observer = server.guest("Observer").await.client;
    expect_ok(
        observer
            .request(Request::JoinLobby(proto::JoinLobby {
                lobby_id: list.lobbies[0].lobby_id.clone(),
                as_observer: true,
            }))
            .await,
    );
    let history = observer.expect(catch_up).await;
    assert_eq!(history.header.unwrap().match_id, match_id);
    assert!(history.frames.iter().any(|frame| !frame.inputs.is_empty()));
    assert!(history.pauses.is_empty());

    let request = || {
        Request::RequestCatchUp(proto::RequestCatchUp {
            match_id: match_id.clone(),
        })
    };
    observer.send(request()).await;
    observer.expect(catch_up).await;
    expect_error(
        observer.request(request()).await,
        proto::ErrorCode::RateLimited,
    );
}

async fn play_tictactoe_to_the_end(host: &mut Client, guest: &mut Client) -> String {
    let match_id = start_two_player_tictactoe(host, guest).await;
    for (x, y) in [(0, 0), (0, 1), (0, 2)] {
        expect_ok(host.request(place(&match_id, x, y)).await);
        if y < 2 {
            expect_ok(guest.request(place(&match_id, 1, y)).await);
        }
    }
    host.expect(match_ended).await;
    guest.expect(match_ended).await;
    match_id
}

fn post_match(message: &proto::ServerMessage) -> Option<proto::LobbyState> {
    lobby_state(message).filter(|l| l.phase == proto::LobbyPhase::PostMatch as i32)
}

#[tokio::test]
async fn post_match_reports_the_result_and_everyone_voting_starts_the_next_match() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    let match_id = play_tictactoe_to_the_end(&mut host, &mut guest).await;
    let state = host.expect(post_match).await;
    assert_eq!(state.last_match_id, match_id);
    assert_eq!(state.last_result.unwrap().winner_slots, vec![0]);

    expect_ok(
        host.request(Request::VotePlayAgain(proto::VotePlayAgain {}))
            .await,
    );
    expect_ok(
        guest
            .request(Request::VotePlayAgain(proto::VotePlayAgain {}))
            .await,
    );
    let next = host.expect(match_started).await;
    assert_ne!(next.match_id, match_id);
}

#[tokio::test]
async fn a_player_leaving_after_a_match_asks_the_others_to_vote_again() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    play_tictactoe_to_the_end(&mut host, &mut guest).await;
    host.expect(post_match).await;
    expect_ok(
        host.request(Request::VotePlayAgain(proto::VotePlayAgain {}))
            .await,
    );
    assert_eq!(host.expect(play_again_votes).await, 1);

    expect_ok(
        guest
            .request(Request::LeaveLobby(proto::LeaveLobby {}))
            .await,
    );

    // Not a silent start at a smaller table: the vote is asked again.
    let state = host
        .expect(|m| post_match(m).filter(|state| state.seats.len() == 1))
        .await;
    assert!(state.play_again_votes.is_empty());
}

/// Wall deaths on a small field: a snake match ends within a few ticks.
fn short_snake() -> proto::GameSettings {
    use mg_protocol::WireGame;
    mg_snake::Snake::settings_to_wire(&mg_snake::RawSettings {
        field_width: 10,
        field_height: 10,
        wall_collision_mode: mg_snake::WallCollisionMode::Death,
        tick_interval_ms: 50,
        ..mg_snake::RawSettings::default()
    })
}

#[tokio::test]
async fn the_ones_left_can_play_again_after_voting_again() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    let lobby = create_lobby(&mut host, short_snake(), 2).await;
    join_ready(&mut guest, &lobby.lobby_id).await;
    let first = start(&mut host).await;
    host.expect(match_ended).await;
    host.expect(post_match).await;
    expect_ok(
        host.request(Request::VotePlayAgain(proto::VotePlayAgain {}))
            .await,
    );
    expect_ok(
        guest
            .request(Request::LeaveLobby(proto::LeaveLobby {}))
            .await,
    );
    host.expect(|m| post_match(m).filter(|state| state.seats.len() == 1))
        .await;
    expect_ok(
        host.request(Request::VotePlayAgain(proto::VotePlayAgain {}))
            .await,
    );
    let next = host.expect(match_started).await;
    assert_ne!(next.match_id, first.match_id);
    assert_eq!(next.seats.len(), 1);
}

#[tokio::test]
async fn the_host_holds_slot_zero_when_a_match_starts() {
    let server = TestServer::start().await;
    let mut first_host = server.guest("First").await.client;
    let mut next_host = server.guest("Next").await.client;
    let lobby = create_lobby(&mut first_host, short_snake(), 3).await;
    let efficient = proto::BotKind {
        game: Some(proto::bot_kind::Game::Snake(
            proto::SnakeBotKind::Efficient as i32,
        )),
    };
    expect_ok(
        first_host
            .request(Request::AddBot(proto::AddBot {
                bot_kind: Some(efficient),
            }))
            .await,
    );
    join_ready(&mut next_host, &lobby.lobby_id).await;
    start(&mut first_host).await;
    // Seats are now [First, Bot, Next]; the host leaves mid-match and Next takes over.
    expect_ok(
        first_host
            .request(Request::LeaveLobby(proto::LeaveLobby {}))
            .await,
    );
    next_host.expect(match_ended).await;
    next_host.expect(post_match).await;
    let header = start(&mut next_host).await;
    assert_eq!(
        header.seats[0].player.as_ref().unwrap().display_name,
        "Next"
    );
}

#[tokio::test]
async fn real_time_bots_play_every_tick() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let settings = {
        use mg_protocol::WireGame;
        mg_snake::Snake::settings_to_wire(&mg_snake::RawSettings {
            field_width: 40,
            field_height: 40,
            tick_interval_ms: 50,
            ..mg_snake::RawSettings::default()
        })
    };
    create_lobby(&mut host, settings, 2).await;
    let efficient = proto::BotKind {
        game: Some(proto::bot_kind::Game::Snake(
            proto::SnakeBotKind::Efficient as i32,
        )),
    };
    expect_ok(
        host.request(Request::AddBot(proto::AddBot {
            bot_kind: Some(efficient),
        }))
        .await,
    );
    start(&mut host).await;
    host.expect(|m| {
        match &m.message {
        Some(Reply::Frame(frame))
            if frame.inputs.iter().any(|input| {
                matches!(&input.input, Some(proto::frame_input::Input::Command(c)) if c.slot == 1)
            }) =>
        {
            Some(())
        }
        _ => None,
    }
    })
    .await;
}

#[tokio::test]
async fn a_shared_replay_reaches_late_joiners_with_its_controls_until_stopped() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    let match_id = play_tictactoe_to_the_end(&mut host, &mut guest).await;
    let state = host.expect(post_match).await;
    let Reply::ReplayData(replay) = host
        .request(Request::RequestReplay(proto::RequestReplay {
            match_id: match_id.clone(),
        }))
        .await
    else {
        panic!("expected replay data");
    };
    expect_error(
        host.request(Request::RequestReplay(proto::RequestReplay { match_id }))
            .await,
        proto::ErrorCode::RateLimited,
    );

    expect_ok(
        host.request(Request::ShareReplay(proto::ShareReplay {
            replay_file: replay.replay_file.clone(),
            host_only_control: true,
        }))
        .await,
    );
    let shared = guest
        .expect(|m| match &m.message {
            Some(Reply::SharedReplay(shared)) => Some(shared.clone()),
            _ => None,
        })
        .await;
    assert_eq!(shared.replay_file, replay.replay_file);

    let control = |action: proto::ReplayControlAction, tick: u64, speed_percent: u32| {
        Request::ReplayControl(proto::ReplayControl {
            action: action as i32,
            tick,
            speed_percent,
        })
    };
    expect_ok(
        host.request(control(proto::ReplayControlAction::SetSpeed, 0, 200))
            .await,
    );
    expect_ok(
        host.request(control(proto::ReplayControlAction::Seek, 1, 0))
            .await,
    );
    expect_error(
        host.request(control(proto::ReplayControlAction::SetSpeed, 0, 1000))
            .await,
        proto::ErrorCode::InvalidRequest,
    );
    expect_error(
        host.request(control(proto::ReplayControlAction::Seek, 1_000_000, 0))
            .await,
        proto::ErrorCode::InvalidRequest,
    );
    expect_error(
        guest
            .request(control(proto::ReplayControlAction::Pause, 0, 0))
            .await,
        proto::ErrorCode::NotHost,
    );

    let mut late = server.guest("Late").await.client;
    expect_ok(
        late.request(Request::JoinLobby(proto::JoinLobby {
            lobby_id: state.lobby_id,
            as_observer: true,
        }))
        .await,
    );
    let mut received = Vec::new();
    while received.len() < 3 {
        let message = late.receive().await;
        match message.message {
            Some(Reply::SharedReplay(shared)) => {
                received.push(format!("file {}", shared.replay_file.len()))
            }
            Some(Reply::ReplayControl(control)) => {
                received.push(format!(
                    "control {} {} {}",
                    control.action, control.tick, control.speed_percent
                ));
            }
            _ => {}
        }
    }
    assert_eq!(
        received,
        vec![
            format!("file {}", replay.replay_file.len()),
            format!(
                "control {} 0 200",
                proto::ReplayControlAction::SetSpeed as i32
            ),
            format!("control {} 1 0", proto::ReplayControlAction::Seek as i32),
        ]
    );

    expect_ok(
        host.request(control(proto::ReplayControlAction::Stop, 0, 0))
            .await,
    );
    late.expect(|m| lobby_state(m).filter(|l| l.phase == proto::LobbyPhase::Open as i32))
        .await;
}

#[tokio::test]
async fn an_idle_lobby_watching_a_replay_is_closed() {
    let server = TestServer::start_with(|config| {
        config.timing = quick_timing();
        config.timing.idle_lobby_timeout = Duration::from_millis(500);
        config.timing.housekeeping_every = Duration::from_millis(100);
    })
    .await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    let match_id = play_tictactoe_to_the_end(&mut host, &mut guest).await;
    let Reply::ReplayData(replay) = host
        .request(Request::RequestReplay(proto::RequestReplay { match_id }))
        .await
    else {
        panic!("expected replay data");
    };
    expect_ok(
        host.request(Request::ShareReplay(proto::ShareReplay {
            replay_file: replay.replay_file,
            host_only_control: false,
        }))
        .await,
    );
    host.expect(|m| lobby_state(m).filter(|l| l.phase == proto::LobbyPhase::Replay as i32))
        .await;
    let closed = host
        .expect(|m| match &m.message {
            Some(Reply::LobbyClosed(closed)) => Some(*closed),
            _ => None,
        })
        .await;
    assert_eq!(closed.reason, proto::LobbyClosedReason::Inactive as i32);
}

#[tokio::test]
async fn chat_and_lobby_names_lose_invisible_characters() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let created = host
        .request(Request::CreateLobby(proto::CreateLobby {
            name: "Fun\u{2067} room\u{00AD}".into(),
            settings: Some(tictactoe_settings(3, 3)),
            max_seats: 0,
        }))
        .await;
    let Reply::LobbyState(lobby) = created else {
        panic!("create failed: {created:?}");
    };
    assert_eq!(lobby.name, "Fun room");
    host.send(Request::SendChat(proto::SendChat {
        scope: proto::ChatScope::Lobby as i32,
        text: "h\u{2066}i\u{3164}".into(),
    }))
    .await;
    let chat = host
        .expect(|m| match &m.message {
            Some(Reply::Chat(chat)) => Some(chat.clone()),
            _ => None,
        })
        .await;
    assert_eq!(chat.text, "hi");
}

#[tokio::test]
async fn dropping_a_seat_during_a_pause_does_not_advance_a_real_time_match() {
    let server = TestServer::start_with(|config| {
        config.timing = quick_timing();
        config.timing.drop_vote_after = Duration::ZERO;
    })
    .await;
    let mut host = server.guest("Host").await.client;
    let mut first = server.guest("First").await.client;
    let mut second = server.guest("Second").await.client;
    let lobby = create_lobby(&mut host, snake_settings(), 3).await;
    join_ready(&mut first, &lobby.lobby_id).await;
    join_ready(&mut second, &lobby.lobby_id).await;
    let match_id = start(&mut host).await.match_id;

    drop(first);
    drop(second);
    // Frames sealed before the pause arrive before it, on the same connection.
    let mut last_tick = 0;
    host.expect(|message| match &message.message {
        Some(Reply::Frame(frame)) => {
            last_tick = last_tick.max(frame.tick);
            None
        }
        _ => paused_seats(message).filter(|seats| seats == &[1, 2]),
    })
    .await;

    expect_ok(
        host.request(Request::VoteDropPlayer(proto::VoteDropPlayer {
            match_id: match_id.clone(),
            slot: 2,
        }))
        .await,
    );
    host.send(Request::RequestCatchUp(proto::RequestCatchUp {
        match_id: match_id.clone(),
    }))
    .await;
    let caught_up = host.expect(catch_up).await;

    assert_eq!(
        caught_up.next_tick,
        last_tick + 1,
        "the paused match moved on"
    );
}

fn play_again_votes(message: &proto::ServerMessage) -> Option<usize> {
    post_match(message).map(|state| state.play_again_votes.len())
}

#[tokio::test]
async fn changing_the_settings_after_a_match_asks_for_new_play_again_votes() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    play_tictactoe_to_the_end(&mut host, &mut guest).await;
    host.expect(post_match).await;
    expect_ok(
        host.request(Request::VotePlayAgain(proto::VotePlayAgain {}))
            .await,
    );
    assert_eq!(host.expect(play_again_votes).await, 1);

    expect_ok(
        host.request(Request::UpdateLobbySettings(proto::UpdateLobbySettings {
            settings: Some(tictactoe_settings(5, 4)),
            max_seats: 2,
        }))
        .await,
    );

    assert_eq!(host.expect(play_again_votes).await, 0);
}

#[tokio::test]
async fn a_player_stepping_aside_to_watch_asks_for_new_votes() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    let lobby = create_lobby(&mut host, short_snake(), 2).await;
    join_ready(&mut guest, &lobby.lobby_id).await;
    let first = start(&mut host).await;
    host.expect(match_ended).await;
    host.expect(post_match).await;
    expect_ok(
        host.request(Request::VotePlayAgain(proto::VotePlayAgain {}))
            .await,
    );

    expect_ok(
        guest
            .request(Request::SwitchToObserver(proto::SwitchToObserver {}))
            .await,
    );
    let state = host
        .expect(|m| post_match(m).filter(|state| state.seats.len() == 1))
        .await;
    assert!(state.play_again_votes.is_empty());

    expect_ok(
        host.request(Request::VotePlayAgain(proto::VotePlayAgain {}))
            .await,
    );
    let next = host.expect(match_started).await;
    assert_ne!(next.match_id, first.match_id);
    assert_eq!(next.seats.len(), 1);
}

#[tokio::test]
async fn removing_a_bot_after_a_match_asks_for_new_votes() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    let lobby = create_lobby(&mut host, short_snake(), 3).await;
    join_ready(&mut guest, &lobby.lobby_id).await;
    let efficient = proto::BotKind {
        game: Some(proto::bot_kind::Game::Snake(
            proto::SnakeBotKind::Efficient as i32,
        )),
    };
    expect_ok(
        host.request(Request::AddBot(proto::AddBot {
            bot_kind: Some(efficient),
        }))
        .await,
    );
    start(&mut host).await;
    host.expect(match_ended).await;
    host.expect(post_match).await;
    expect_ok(
        guest
            .request(Request::VotePlayAgain(proto::VotePlayAgain {}))
            .await,
    );
    assert_eq!(host.expect(play_again_votes).await, 1);

    expect_ok(
        host.request(Request::RemoveSeat(proto::RemoveSeat { seat: 2 }))
            .await,
    );

    assert_eq!(host.expect(play_again_votes).await, 0);
}

#[tokio::test]
async fn sharing_a_replay_is_answered_even_when_the_host_moved_on_meanwhile() {
    let server = TestServer::start().await;
    let mut host = server.guest("Host").await.client;
    let mut guest = server.guest("Guest").await.client;
    let match_id = play_tictactoe_to_the_end(&mut host, &mut guest).await;
    host.expect(post_match).await;
    let Reply::ReplayData(replay) = host
        .request(Request::RequestReplay(proto::RequestReplay { match_id }))
        .await
    else {
        panic!("expected replay data");
    };

    // Sent back to back: the replay is still being checked while the host goes elsewhere.
    let share = host
        .send(Request::ShareReplay(proto::ShareReplay {
            replay_file: replay.replay_file,
            host_only_control: true,
        }))
        .await;
    host.send(Request::LeaveLobby(proto::LeaveLobby {})).await;
    host.send(Request::CreateLobby(proto::CreateLobby {
        name: "elsewhere".into(),
        settings: Some(tictactoe_settings(3, 3)),
        max_seats: 2,
    }))
    .await;

    expect_error(host.reply_to(share).await, proto::ErrorCode::NotInLobby);
}
