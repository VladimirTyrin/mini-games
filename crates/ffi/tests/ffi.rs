//! Calls the exported functions the way a foreign caller does, valid and invalid input alike.

#![allow(unsafe_code)]

use std::ptr;

use mg_protocol::{WireGame, proto};
use mg_tictactoe::{Command, FirstPlayerMode, Point, RawSettings, TicTacToe};
use mini_games::*;
use prost::Message;

fn take(buffer: MgBuffer) -> Vec<u8> {
    let bytes = if buffer.data.is_null() {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) }.to_vec()
    };
    unsafe { mg_buffer_free(buffer) };
    bytes
}

fn last_error() -> String {
    let mut buffer = MgBuffer {
        data: ptr::null_mut(),
        len: 0,
        capacity: 0,
    };
    assert_eq!(unsafe { mg_last_error_message(&mut buffer) }, MgStatus::Ok);
    String::from_utf8(take(buffer)).unwrap()
}

fn empty_buffer() -> MgBuffer {
    MgBuffer {
        data: ptr::null_mut(),
        len: 0,
        capacity: 0,
    }
}

fn header(seats: Vec<proto::MatchSeat>) -> Vec<u8> {
    let settings = TicTacToe::settings_to_wire(&RawSettings {
        field_width: 3,
        field_height: 3,
        win_count: 3,
        first_player: FirstPlayerMode::Host,
    });
    proto::MatchHeader {
        match_id: "local".into(),
        settings: Some(settings),
        seats,
        ..Default::default()
    }
    .encode_to_vec()
}

fn human(slot: u32) -> proto::MatchSeat {
    proto::MatchSeat {
        slot,
        player: None,
        bot_kind: None,
    }
}

fn random_bot(slot: u32) -> proto::MatchSeat {
    proto::MatchSeat {
        slot,
        player: None,
        bot_kind: Some(proto::BotKind {
            game: Some(proto::bot_kind::Game::Tictactoe(
                proto::TicTacToeBotKind::Random as i32,
            )),
        }),
    }
}

fn two_humans() -> Vec<u8> {
    header(vec![human(0), human(1)])
}

fn place(x: u32, y: u32) -> Vec<u8> {
    TicTacToe::command_to_wire(&Command::Place(Point { x, y })).encode_to_vec()
}

struct Authority(*mut MgAuthority);

impl Authority {
    fn new(header: &[u8]) -> Self {
        let mut handle = ptr::null_mut();
        let status = unsafe { mg_authority_new(header.as_ptr(), header.len(), 7, &mut handle) };
        assert_eq!(status, MgStatus::Ok, "{}", last_error());
        Self(handle)
    }

    fn submit(&self, slot: u32, command: &[u8]) -> MgStatus {
        unsafe { mg_authority_submit(self.0, slot, command.as_ptr(), command.len()) }
    }

    /// Submits and seals; returns the encoded frame.
    fn play(&self, slot: u32, x: u32, y: u32) -> Vec<u8> {
        assert_eq!(
            self.submit(slot, &place(x, y)),
            MgStatus::Ok,
            "{}",
            last_error()
        );
        self.seal()
    }

    fn seal(&self) -> Vec<u8> {
        let mut frame = empty_buffer();
        assert_eq!(
            unsafe { mg_authority_seal_frame(self.0, &mut frame) },
            MgStatus::Ok
        );
        take(frame)
    }

    fn view(&self) -> proto::TicTacToeView {
        let mut view = empty_buffer();
        assert_eq!(
            unsafe { mg_authority_view(self.0, -1, &mut view) },
            MgStatus::Ok
        );
        tictactoe_view(&take(view))
    }

    fn result(&self) -> Option<proto::MatchResult> {
        let mut result = empty_buffer();
        assert_eq!(
            unsafe { mg_authority_result(self.0, &mut result) },
            MgStatus::Ok
        );
        let bytes = take(result);
        (!bytes.is_empty()).then(|| proto::MatchResult::decode(bytes.as_slice()).unwrap())
    }

    fn clone_handle(&self) -> Self {
        let mut handle = ptr::null_mut();
        assert_eq!(
            unsafe { mg_authority_clone(self.0, &mut handle) },
            MgStatus::Ok
        );
        Self(handle)
    }
}

impl Drop for Authority {
    fn drop(&mut self) {
        unsafe { mg_authority_free(self.0) };
    }
}

struct Mirror(*mut MgMirror);

impl Mirror {
    fn new(header: &[u8]) -> Self {
        let mut handle = ptr::null_mut();
        let status = unsafe { mg_mirror_new(header.as_ptr(), header.len(), &mut handle) };
        assert_eq!(status, MgStatus::Ok, "{}", last_error());
        Self(handle)
    }

    fn apply(&self, frame: &[u8]) -> MgStatus {
        unsafe { mg_mirror_apply_frame(self.0, frame.as_ptr(), frame.len()) }
    }

    fn view(&self) -> proto::TicTacToeView {
        let mut view = empty_buffer();
        assert_eq!(
            unsafe { mg_mirror_view(self.0, 0, &mut view) },
            MgStatus::Ok
        );
        tictactoe_view(&take(view))
    }

    fn result(&self) -> Option<proto::MatchResult> {
        let mut result = empty_buffer();
        assert_eq!(
            unsafe { mg_mirror_result(self.0, &mut result) },
            MgStatus::Ok
        );
        let bytes = take(result);
        (!bytes.is_empty()).then(|| proto::MatchResult::decode(bytes.as_slice()).unwrap())
    }
}

impl Drop for Mirror {
    fn drop(&mut self) {
        unsafe { mg_mirror_free(self.0) };
    }
}

fn tictactoe_view(bytes: &[u8]) -> proto::TicTacToeView {
    match proto::GameView::decode(bytes).unwrap().game {
        Some(proto::game_view::Game::Tictactoe(view)) => view,
        other => panic!("not a TicTacToe view: {other:?}"),
    }
}

fn mark_count(view: &proto::TicTacToeView) -> usize {
    view.cells.iter().filter(|cell| **cell != 0).count()
}

#[test]
fn a_local_match_streams_frames_that_a_mirror_replays() {
    let header = two_humans();
    let authority = Authority::new(&header);
    let mirror = Mirror::new(&header);
    // X (slot 0) completes the top row.
    for (slot, x, y) in [(0, 0, 0), (1, 0, 1), (0, 1, 0), (1, 1, 1), (0, 2, 0)] {
        let frame = authority.play(slot, x, y);
        assert_eq!(mirror.apply(&frame), MgStatus::Ok, "{}", last_error());
    }
    let mut finished = false;
    assert_eq!(
        unsafe { mg_mirror_is_finished(mirror.0, &mut finished) },
        MgStatus::Ok
    );
    assert!(finished);
    let result = authority.result().unwrap();
    assert_eq!(result.winner_slots, vec![0]);
    assert_eq!(mirror.result(), Some(result));
    assert_eq!(mirror.view().cells, authority.view().cells);
    assert_eq!(mirror.view().you, Some(0));
}

#[test]
fn rejected_commands_change_nothing_and_say_why() {
    let authority = Authority::new(&two_humans());
    authority.play(0, 1, 1);
    assert_eq!(authority.submit(1, &place(1, 1)), MgStatus::Rejected);
    assert!(last_error().contains("rejected"), "{}", last_error());
    let mut buffer = empty_buffer();
    assert_eq!(unsafe { mg_last_refusal(&mut buffer) }, MgStatus::Ok);
    let refusal = take(buffer);
    assert_eq!(
        proto::Refusal::decode(refusal.as_slice()).unwrap().reason,
        Some(proto::refusal::Reason::Tictactoe(
            proto::TicTacToeRefusal::CellTaken.into()
        ))
    );
    let mut buffer = empty_buffer();
    assert_eq!(
        unsafe { mg_refusal_english(refusal.as_ptr(), refusal.len(), &mut buffer) },
        MgStatus::Ok
    );
    assert_eq!(take(buffer), b"cell is already marked");
    assert_eq!(authority.submit(0, &place(0, 0)), MgStatus::Rejected);
    assert_eq!(authority.submit(9, &place(0, 0)), MgStatus::Rejected);
    assert_eq!(
        authority.submit(300, &place(0, 0)),
        MgStatus::InvalidArgument
    );
    assert_eq!(mark_count(&authority.view()), 1);
}

#[test]
fn a_clone_continues_independently() {
    let original = Authority::new(&two_humans());
    original.play(0, 1, 1);
    original.play(1, 0, 0);
    let copy = original.clone_handle();
    assert_eq!(copy.view().cells, original.view().cells);

    copy.play(0, 2, 2);
    assert_eq!(mark_count(&copy.view()), 3);
    assert_eq!(mark_count(&original.view()), 2);
    // The same move is still free in the original.
    original.play(0, 2, 2);
    assert_eq!(copy.view().cells, original.view().cells);
}

#[test]
fn built_in_bots_play_their_seats() {
    let authority = Authority::new(&header(vec![human(0), random_bot(1)]));
    authority.play(0, 1, 1);
    let mut waiting = false;
    assert_eq!(
        unsafe { mg_authority_is_waiting_for_bot(authority.0, &mut waiting) },
        MgStatus::Ok
    );
    assert!(waiting);
    assert_eq!(unsafe { mg_authority_run_bots(authority.0) }, MgStatus::Ok);
    authority.seal();
    assert_eq!(mark_count(&authority.view()), 2);
    assert_eq!(authority.view().current_slot, Some(0));
}

#[test]
fn a_replay_file_replays_to_the_same_result() {
    let authority = Authority::new(&two_humans());
    for (slot, x, y) in [(0, 0, 0), (1, 1, 0), (0, 0, 1), (1, 1, 1), (0, 0, 2)] {
        authority.play(slot, x, y);
    }
    let mut replay = empty_buffer();
    assert_eq!(
        unsafe { mg_authority_replay_file(authority.0, &mut replay) },
        MgStatus::Ok
    );
    let replay = take(replay);

    let mut handle = ptr::null_mut();
    assert_eq!(
        unsafe { mg_mirror_from_replay_file(replay.as_ptr(), replay.len(), &mut handle) },
        MgStatus::Ok
    );
    let mirror = Mirror(handle);
    let mut end = 0;
    assert_eq!(
        unsafe { mg_mirror_replay_end_tick(mirror.0, &mut end) },
        MgStatus::Ok
    );
    assert_eq!(end, 5);
    assert_eq!(unsafe { mg_mirror_advance_to(mirror.0, 2) }, MgStatus::Ok);
    assert_eq!(mark_count(&mirror.view()), 2);
    assert_eq!(unsafe { mg_mirror_advance_to(mirror.0, end) }, MgStatus::Ok);
    assert_eq!(mirror.result(), authority.result());
}

#[test]
fn catch_up_brings_a_new_mirror_up_to_date() {
    let header_bytes = two_humans();
    let authority = Authority::new(&header_bytes);
    let mut frames = Vec::new();
    for (slot, x, y) in [(0, 1, 1), (1, 0, 0)] {
        frames.push(proto::TickFrame::decode(authority.play(slot, x, y).as_slice()).unwrap());
    }
    let catch_up = proto::MatchCatchUp {
        header: Some(proto::MatchHeader::decode(header_bytes.as_slice()).unwrap()),
        frames,
        next_tick: 2,
        ..Default::default()
    }
    .encode_to_vec();
    let mirror = Mirror::new(&header_bytes);
    assert_eq!(
        unsafe { mg_mirror_apply_catch_up(mirror.0, catch_up.as_ptr(), catch_up.len()) },
        MgStatus::Ok
    );
    let mut next_tick = 0;
    assert_eq!(
        unsafe { mg_mirror_next_tick(mirror.0, &mut next_tick) },
        MgStatus::Ok
    );
    assert_eq!(next_tick, 2);
    assert_eq!(mirror.view().cells, authority.view().cells);
}

#[test]
fn out_of_order_and_corrupt_frames_are_desyncs() {
    let header = two_humans();
    let authority = Authority::new(&header);
    let mirror = Mirror::new(&header);
    authority.play(0, 1, 1);
    let second = authority.play(1, 0, 0);
    assert_eq!(mirror.apply(&second), MgStatus::Desync);
    assert!(
        last_error().contains("expected frame 0"),
        "{}",
        last_error()
    );

    let mut wrong_hash = proto::TickFrame::decode(second.as_slice()).unwrap();
    wrong_hash.tick = 0;
    wrong_hash.inputs.clear();
    wrong_hash.state_hash = Some(1);
    let bytes = wrong_hash.encode_to_vec();
    assert_eq!(mirror.apply(&bytes), MgStatus::Desync);
}

#[test]
fn invalid_input_is_reported_not_trusted() {
    let mut handle: *mut MgAuthority = ptr::null_mut();
    let garbage = [0xff_u8, 0x01, 0x02];
    unsafe {
        assert_eq!(
            mg_authority_new(ptr::null(), 4, 1, &mut handle),
            MgStatus::NullArgument
        );
        assert_eq!(
            mg_authority_new(garbage.as_ptr(), garbage.len(), 1, &mut handle),
            MgStatus::InvalidArgument
        );
        assert!(handle.is_null());
        // A header without settings.
        assert_eq!(
            mg_authority_new(ptr::null(), 0, 1, &mut handle),
            MgStatus::InvalidArgument
        );
        let header = two_humans();
        assert_eq!(
            mg_authority_new(header.as_ptr(), header.len(), 1, ptr::null_mut()),
            MgStatus::NullArgument
        );
        assert_eq!(
            mg_authority_submit(ptr::null_mut(), 0, ptr::null(), 0),
            MgStatus::NullArgument
        );
        assert_eq!(
            mg_authority_seal_frame(ptr::null_mut(), ptr::null_mut()),
            MgStatus::NullArgument
        );
        mg_authority_free(ptr::null_mut());
        mg_mirror_free(ptr::null_mut());
        mg_buffer_free(empty_buffer());
    }

    let authority = Authority::new(&two_humans());
    assert_eq!(authority.submit(0, &garbage), MgStatus::InvalidArgument);
    assert_eq!(authority.submit(0, &[]), MgStatus::InvalidArgument);
    assert_eq!(
        unsafe { mg_authority_seal_frame(authority.0, ptr::null_mut()) },
        MgStatus::NullArgument
    );
    let mut view = empty_buffer();
    assert_eq!(
        unsafe { mg_authority_view(authority.0, 300, &mut view) },
        MgStatus::InvalidArgument
    );
    assert!(view.data.is_null());

    let mirror = Mirror::new(&two_humans());
    assert_eq!(mirror.apply(&garbage), MgStatus::InvalidArgument);
    assert_eq!(
        unsafe { mg_mirror_advance_to(mirror.0, 1) },
        MgStatus::InvalidState
    );
    let mut replay_mirror = ptr::null_mut();
    assert_eq!(
        unsafe { mg_mirror_from_replay_file(garbage.as_ptr(), garbage.len(), &mut replay_mirror) },
        MgStatus::InvalidArgument
    );
}

#[test]
fn catalog_answers_from_the_core() {
    let id = b"tictactoe";
    let mut settings = empty_buffer();
    assert_eq!(
        unsafe { mg_default_settings(id.as_ptr(), id.len(), &mut settings) },
        MgStatus::Ok
    );
    let settings = take(settings);
    let decoded = proto::GameSettings::decode(settings.as_slice()).unwrap();
    assert!(matches!(
        decoded.game,
        Some(proto::game_settings::Game::Tictactoe(_))
    ));
    unsafe {
        assert_eq!(
            mg_validate_settings(settings.as_ptr(), settings.len(), 2),
            MgStatus::Ok
        );
        assert_eq!(
            mg_validate_settings(settings.as_ptr(), settings.len(), 3),
            MgStatus::InvalidArgument
        );
    }
    assert!(last_error().contains("players"), "{}", last_error());

    let mut kinds = empty_buffer();
    assert_eq!(
        unsafe { mg_bot_kinds(id.as_ptr(), id.len(), &mut kinds) },
        MgStatus::Ok
    );
    let kinds = take(kinds);
    let mut rest = kinds.as_slice();
    let mut count = 0;
    while !rest.is_empty() {
        proto::BotKind::decode_length_delimited(&mut rest).unwrap();
        count += 1;
    }
    assert_eq!(count, 2);

    let mut unused = empty_buffer();
    let unknown = b"chess";
    let not_utf8 = [0xc3_u8, 0x28];
    unsafe {
        assert_eq!(
            mg_default_settings(unknown.as_ptr(), unknown.len(), &mut unused),
            MgStatus::InvalidArgument
        );
        assert_eq!(
            mg_bot_kinds(not_utf8.as_ptr(), not_utf8.len(), &mut unused),
            MgStatus::InvalidArgument
        );
    }
    assert_eq!(mg_abi_version(), MG_ABI_VERSION);
}

#[test]
fn the_last_error_is_per_thread() {
    let authority = Authority::new(&two_humans());
    assert_eq!(authority.submit(0, &[0xff]), MgStatus::InvalidArgument);
    let other_thread = std::thread::spawn(last_error).join().unwrap();
    assert_eq!(other_thread, "");
    assert!(!last_error().is_empty());
}
