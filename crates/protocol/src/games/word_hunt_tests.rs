use mg_core::{Game, PlayerSpec, Viewer};
use mg_word_hunt::{
    BotKind, Command, Language, PuzzleMode, RawSettings, Settings, WordHunt, WorldEvent,
};
use prost::Message;

use crate::proto;
use crate::wire_error::WireError;
use crate::wire_game::{WireBots, WireGame};

fn binary<M: Message + Default + PartialEq + std::fmt::Debug>(message: &M) -> M {
    let decoded = M::decode(message.encode_to_vec().as_slice()).unwrap();
    assert_eq!(&decoded, message);
    decoded
}

#[test]
fn settings_round_trip_unvalidated() {
    let samples = [
        RawSettings::default(),
        RawSettings {
            language: Language::Russian,
            word_length: 7,
            max_guesses: 8,
            hard_mode: true,
            mode: PuzzleMode::Daily,
            day: 20_400,
        },
        // Rule validation belongs to the game: the wire keeps even invalid values intact.
        RawSettings {
            word_length: u32::MAX,
            max_guesses: 0,
            ..RawSettings::default()
        },
    ];
    for settings in samples {
        let wire = binary(&WordHunt::settings_to_wire(&settings));
        assert_eq!(WordHunt::settings_from_wire(&wire), Ok(settings));
    }
}

#[test]
fn malformed_settings_are_rejected() {
    let mut wire = WordHunt::settings_to_wire(&RawSettings::default());
    let Some(proto::game_settings::Game::WordHunt(settings)) = wire.game.as_mut() else {
        panic!("not word hunt settings");
    };
    settings.language = 0;
    assert_eq!(
        WordHunt::settings_from_wire(&wire),
        Err(WireError::Missing("language"))
    );
    let Some(proto::game_settings::Game::WordHunt(settings)) = wire.game.as_mut() else {
        panic!("not word hunt settings");
    };
    settings.language = 1;
    settings.mode = 99;
    assert_eq!(
        WordHunt::settings_from_wire(&wire),
        Err(WireError::UnknownEnumValue("mode"))
    );
}

#[test]
fn commands_and_world_events_round_trip() {
    for command in [Command::Guess("ёлка".into()), Command::GiveUp] {
        let wire = binary(&WordHunt::command_to_wire(&command));
        assert_eq!(WordHunt::command_from_wire(&wire), Ok(command));
    }
    for event in [
        WorldEvent::AnswerChosen("crane".into()),
        WorldEvent::FirstPlayerChosen(mg_core::Slot::new(2)),
    ] {
        let wire = binary(&WordHunt::world_event_to_wire(&event));
        assert_eq!(WordHunt::world_event_from_wire(&wire), Ok(event));
    }
}

#[test]
fn bot_kinds_round_trip() {
    for kind in [BotKind::Easy, BotKind::Medium, BotKind::Hard] {
        let wire = binary(&WordHunt::bot_kind_to_wire(kind));
        assert_eq!(WordHunt::bot_kind_from_wire(&wire), Ok(kind));
    }
}

#[test]
fn an_overlong_word_is_refused_on_the_wire() {
    let wire = WordHunt::command_to_wire(&Command::Guess("x".repeat(33)));
    assert_eq!(
        WordHunt::command_from_wire(&wire),
        Err(WireError::OutOfRange("word"))
    );
}

#[test]
fn view_carries_guesses_marks_and_the_revealed_answer() {
    let mut game = WordHunt::new(&Settings::default(), &[PlayerSpec::HUMAN]).unwrap();
    game.apply_world(&WorldEvent::AnswerChosen("crane".into()))
        .unwrap();
    game.apply_command(mg_core::Slot::new(0), &Command::Guess("cable".into()))
        .unwrap();
    let Some(proto::game_view::Game::WordHunt(view)) =
        binary(&WordHunt::view_to_wire(&game.view(Viewer::Spectator))).game
    else {
        panic!("not a word hunt view");
    };
    assert_eq!(view.guesses.len(), 1);
    assert_eq!(
        view.guesses[0].marks,
        [3, 2, 1, 1, 3].map(i32::from).to_vec(),
        "correct, present, absent, absent, correct"
    );
    assert_eq!(view.answer, None);
    game.apply_command(mg_core::Slot::new(0), &Command::Guess("crane".into()))
        .unwrap();
    let Some(proto::game_view::Game::WordHunt(view)) =
        WordHunt::view_to_wire(&game.view(Viewer::Spectator)).game
    else {
        panic!("not a word hunt view");
    };
    assert_eq!(view.answer.as_deref(), Some("crane"));
    assert_eq!(view.phase, i32::from(proto::WordHuntPhase::Solved));
}

#[cfg(feature = "json")]
#[test]
fn authority_frames_replay_through_json() {
    let settings = <Settings as mg_core::GameSettings>::parse(RawSettings {
        language: Language::Russian,
        ..RawSettings::default()
    })
    .unwrap();
    let players = [PlayerSpec::HUMAN];
    let mut authority =
        mg_core::Authority::<WordHunt>::new(&settings, &players, 5, vec![]).unwrap();
    let mut client = mg_core::Match::<WordHunt>::new(&settings, &players).unwrap();
    let mut deliver = |frame: &mg_core::GameFrame<WordHunt>| {
        let json = serde_json::to_string(&crate::frame_to_wire::<WordHunt>(frame)).unwrap();
        let wire: proto::TickFrame = serde_json::from_str(&json).unwrap();
        client
            .apply_frame(&crate::frame_from_wire::<WordHunt>(&wire).unwrap())
            .unwrap();
    };
    deliver(&authority.seal_frame());
    for guess in ["сокол", "КОШКА", "ёжики", "банан", "лампа", "ручка"]
    {
        authority
            .submit(mg_core::Slot::new(0), Command::Guess(guess.into()))
            .unwrap();
        deliver(&authority.seal_frame());
        if authority.game().status().is_finished() {
            break;
        }
    }
    assert!(client.game().status().is_finished());
    assert_eq!(client.game().state_hash(), authority.game().state_hash());
}
