use mg_checkers::{
    BotKind, Checkers, Command, Path, RawSettings, Settings, Square, WhitePlayer, WorldEvent,
};
use mg_core::{
    Authority, Awaiting, BotSeat, Game, GameSettings, HasBots, Match, PlayerSpec, Slot, Viewer,
};
use prost::Message;

use crate::proto::{self, CheckersSide};
use crate::{WireBots, WireError, WireGame, frame_from_wire, frame_to_wire};

fn wire_move(path: Vec<u32>) -> proto::GameCommand {
    proto::GameCommand {
        game: Some(proto::game_command::Game::Checkers(
            proto::CheckersCommand {
                command: Some(proto::checkers_command::Command::Move(
                    proto::CheckersMove { path },
                )),
            },
        )),
    }
}

fn checkers_view(view: &mg_checkers::View) -> proto::CheckersView {
    match Checkers::view_to_wire(view).game {
        Some(proto::game_view::Game::Checkers(view)) => view,
        other => panic!("not a checkers view: {other:?}"),
    }
}

#[test]
fn settings_round_trip() {
    for white_player in [WhitePlayer::Random, WhitePlayer::Host, WhitePlayer::Guest] {
        let original = RawSettings { white_player };
        let wire = Checkers::settings_to_wire(&original);
        assert_eq!(Checkers::settings_from_wire(&wire), Ok(original));
    }
}

#[test]
fn commands_and_world_events_round_trip() {
    let path = Path::new(vec![
        Square::new(2, 2).unwrap(),
        Square::new(4, 4).unwrap(),
        Square::new(6, 6).unwrap(),
    ])
    .unwrap();
    let command = Command::Move(path);
    assert_eq!(
        Checkers::command_from_wire(&Checkers::command_to_wire(&command)),
        Ok(command)
    );
    let event = WorldEvent::WhiteChosen(Slot::new(1));
    assert_eq!(
        Checkers::world_event_from_wire(&Checkers::world_event_to_wire(&event)),
        Ok(event)
    );
}

#[test]
fn bot_kinds_round_trip() {
    for &kind in Checkers::BOT_KINDS {
        let wire = Checkers::bot_kind_to_wire(kind);
        assert_eq!(Checkers::bot_kind_from_wire(&wire), Ok(kind));
    }
}

#[test]
fn malformed_moves_are_rejected_before_anything_is_built() {
    // A light square, off the board, a lone square, a path longer than any move can be.
    for path in [
        vec![1, 18],
        vec![0, 64],
        vec![0],
        vec![0; Path::MAX_SQUARES + 1],
        vec![0; 100_000],
    ] {
        assert_eq!(
            Checkers::command_from_wire(&wire_move(path)),
            Err(WireError::OutOfRange("path"))
        );
    }
}

#[test]
fn unknown_or_missing_white_player_is_rejected() {
    for value in [0, 99] {
        let wire = proto::GameSettings {
            game: Some(proto::game_settings::Game::Checkers(
                proto::CheckersSettings {
                    white_player: value,
                },
            )),
        };
        assert!(Checkers::settings_from_wire(&wire).is_err());
    }
}

#[test]
fn view_carries_pieces_players_and_moves() {
    let game = Checkers::new(
        &Settings {
            white_player: WhitePlayer::Host,
        },
        &[PlayerSpec::HUMAN; 2],
    )
    .unwrap();

    let view = checkers_view(&game.view(Viewer::Player(Slot::new(1))));

    assert_eq!(view.pieces.len(), 24);
    assert_eq!(view.white_slot, Some(0));
    assert_eq!(view.black_slot, Some(1));
    assert_eq!(view.current_slot, Some(0));
    assert_eq!(view.you, Some(1));
    assert_eq!(view.side_to_move, CheckersSide::White as i32);
    assert_eq!(view.legal_moves.len(), 7);
    assert!(view.end.is_none());
}

/// Every sealed frame goes through the wire (binary protobuf, and protobuf JSON with the `json`
/// feature) before a `Match` applies it, like on a real client.
fn mirror_through_the_wire(white_player: WhitePlayer, players: [PlayerSpec; 2], seed: u64) {
    let raw = RawSettings { white_player };
    let settings = Settings::parse(raw).unwrap();
    let wire_settings = Checkers::settings_to_wire(&raw);
    let bots = Slot::all(2)
        .filter(|slot| !players[slot.index()].is_human())
        .map(|slot| BotSeat {
            slot,
            bot: Checkers::make_bot(BotKind::Medium),
            budget_nodes: 300,
        })
        .collect();
    let mut authority = Authority::<Checkers>::new(&settings, &players, seed, bots).unwrap();
    let mut mirror = Match::<Checkers>::new(
        &Settings::parse(Checkers::settings_from_wire(&wire_settings).unwrap()).unwrap(),
        &players,
    )
    .unwrap();
    let mut frames = 0;
    loop {
        let frame = authority.seal_frame();
        let wire =
            proto::TickFrame::decode(frame_to_wire::<Checkers>(&frame).encode_to_vec().as_slice())
                .unwrap();
        #[cfg(feature = "json")]
        let wire: proto::TickFrame =
            serde_json::from_str(&serde_json::to_string(&wire).unwrap()).unwrap();
        mirror
            .apply_frame(&frame_from_wire::<Checkers>(&wire).unwrap())
            .unwrap();
        frames += 1;
        if authority.game().status().is_finished() || frames > 3_000 {
            break;
        }
        match authority.game().awaiting() {
            Awaiting::Turn(slot) if players[slot.index()].is_human() => {
                let first = authority.game().legal_moves().swap_remove(0);
                authority.submit(slot, Command::Move(first.path)).unwrap();
            }
            _ => authority.run_bots(),
        }
    }
    assert!(authority.game().status().is_finished());
    assert_eq!(mirror.game().state_hash(), authority.game().state_hash());
    assert_eq!(mirror.game().end(), authority.game().end());
}

#[test]
fn matches_replay_identically_through_the_wire() {
    for seed in 0..3 {
        mirror_through_the_wire(
            WhitePlayer::Random,
            [PlayerSpec::HUMAN, PlayerSpec::BOT],
            seed,
        );
        mirror_through_the_wire(WhitePlayer::Host, [PlayerSpec::BOT, PlayerSpec::BOT], seed);
    }
}

#[cfg(feature = "json")]
#[test]
fn json_uses_enum_names() {
    let json = serde_json::to_value(Checkers::settings_to_wire(&RawSettings::default())).unwrap();
    assert_eq!(
        json["checkers"]["whitePlayer"],
        "CHECKERS_WHITE_PLAYER_RANDOM"
    );
    let bot = serde_json::to_value(Checkers::bot_kind_to_wire(BotKind::Hard)).unwrap();
    assert_eq!(bot["checkers"], "CHECKERS_BOT_KIND_HARD");
}
