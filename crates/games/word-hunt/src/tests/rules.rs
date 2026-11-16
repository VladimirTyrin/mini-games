use mg_core::{
    Authority, CommandError, Game, GameSettings, OutcomeKind, PlayerSpec, Slot, Status, Viewer,
};

use crate::model::Answers;
use crate::{
    Command, DayNumber, KeyHint, Language, LetterMark, Phase, PuzzleMode, RawSettings, Settings,
    Word, WordError, WordHunt, WordLength, WorldEvent,
};

const PLAYER: Slot = Slot::new(0);
use LetterMark::{Absent, Correct, Present};

fn settings(language: Language, word_length: u32, hard_mode: bool) -> Settings {
    Settings::parse(RawSettings {
        language,
        word_length,
        hard_mode,
        ..RawSettings::default()
    })
    .unwrap()
}

/// A started game whose answer is `answer`.
fn game_with_answer(answer: &str, language: Language, hard_mode: bool) -> WordHunt {
    let length = u32::try_from(answer.chars().count()).unwrap();
    let mut game =
        WordHunt::new(&settings(language, length, hard_mode), &[PlayerSpec::HUMAN]).unwrap();
    game.apply_world(&WorldEvent::AnswerChosen(answer.into()))
        .unwrap();
    game
}

fn guess(game: &mut WordHunt, text: &str) -> Result<(), CommandError> {
    game.apply_command(PLAYER, &Command::Guess(text.into()))
}

fn last_marks(game: &WordHunt) -> Vec<LetterMark> {
    game.view(Viewer::Player(PLAYER))
        .guesses
        .last()
        .unwrap()
        .marks
        .clone()
}

#[test]
fn marks_letters_in_place_elsewhere_and_absent() {
    let mut game = game_with_answer("crane", Language::English, false);
    guess(&mut game, "cable").unwrap();
    assert_eq!(
        last_marks(&game),
        [Correct, Present, Absent, Absent, Correct]
    );
}

#[test]
fn a_repeated_letter_is_marked_only_as_often_as_the_answer_has_it() {
    let mut game = game_with_answer("abide", Language::English, false);
    guess(&mut game, "speed").unwrap();
    assert_eq!(
        last_marks(&game),
        [Absent, Absent, Present, Absent, Present]
    );

    let mut game = game_with_answer("eerie", Language::English, false);
    guess(&mut game, "geese").unwrap();
    assert_eq!(
        last_marks(&game),
        [Absent, Correct, Present, Absent, Correct]
    );
}

#[test]
fn an_exact_match_takes_the_letter_before_an_earlier_misplaced_one() {
    let mut game = game_with_answer("lever", Language::English, false);
    guess(&mut game, "eerie").unwrap();
    // The second `e` is in place, so the answer's other `e` is left for the first one.
    assert_eq!(
        last_marks(&game),
        [Present, Correct, Present, Absent, Absent]
    );
}

#[test]
fn russian_guesses_ignore_case_and_fold_yo_into_ye() {
    let mut game = game_with_answer("ёлка", Language::Russian, false);
    guess(&mut game, "ЕЛКА").unwrap();
    assert_eq!(game.view(Viewer::Spectator).phase, Phase::Solved);
}

#[test]
fn a_malformed_guess_is_rejected_without_using_a_try() {
    let mut game = game_with_answer("crane", Language::English, false);
    for text in ["cran", "cranes", "cr4ne", "кошка", ""] {
        assert!(
            matches!(guess(&mut game, text), Err(CommandError::Illegal(_))),
            "{text}"
        );
    }
    assert!(game.view(Viewer::Spectator).guesses.is_empty());
    assert_eq!(
        Word::parse("crane!", Language::English, WordLength::constant::<5>()),
        Err(WordError::WrongLength)
    );
}

#[test]
fn solving_ends_the_game_with_spare_guesses_as_score() {
    let mut game = game_with_answer("crane", Language::English, false);
    guess(&mut game, "cable").unwrap();
    guess(&mut game, "crane").unwrap();
    let Status::Finished(outcome) = game.status() else {
        panic!("not finished");
    };
    assert_eq!(outcome.kind, OutcomeKind::Victory);
    assert_eq!(outcome.scores, [5]);
    assert_eq!(guess(&mut game, "crane"), Err(CommandError::GameOver));
    assert_eq!(
        game.view(Viewer::Spectator).answer.as_deref(),
        Some("crane")
    );
}

#[test]
fn running_out_of_guesses_loses_and_reveals_the_answer() {
    let mut game = game_with_answer("crane", Language::English, false);
    for _ in 0..5 {
        guess(&mut game, "plumb").unwrap();
        assert_eq!(game.view(Viewer::Spectator).answer, None);
    }
    guess(&mut game, "plumb").unwrap();
    let view = game.view(Viewer::Spectator);
    assert_eq!(view.phase, Phase::Failed);
    assert_eq!(view.answer.as_deref(), Some("crane"));
    assert!(
        matches!(game.status(), Status::Finished(outcome) if outcome.kind == OutcomeKind::Defeat)
    );
}

#[test]
fn hard_mode_requires_found_letters() {
    let mut game = game_with_answer("crane", Language::English, true);
    guess(&mut game, "cable").unwrap();
    // `c` and `e` were found in place, `a` elsewhere.
    assert_eq!(
        guess(&mut game, "brine"),
        Err(CommandError::Illegal(
            "hard mode: keep every green letter in its place"
        ))
    );
    assert_eq!(
        guess(&mut game, "chime"),
        Err(CommandError::Illegal(
            "hard mode: use every letter you have found"
        ))
    );
    assert_eq!(game.view(Viewer::Spectator).guesses.len(), 1);
    guess(&mut game, "chase").unwrap();
}

#[test]
fn keyboard_shows_each_guessed_letters_best_mark() {
    let mut game = game_with_answer("crane", Language::English, false);
    guess(&mut game, "nacre").unwrap();
    guess(&mut game, "crown").unwrap();
    let keyboard = game.view(Viewer::Spectator).keyboard;
    let mark_of = |letter| {
        keyboard
            .iter()
            .find(|hint| hint.letter == letter)
            .map(|hint| hint.mark)
    };
    assert_eq!(mark_of('n'), Some(Present));
    assert_eq!(mark_of('c'), Some(Correct));
    assert_eq!(mark_of('w'), Some(Absent));
    assert_eq!(mark_of('z'), None);
    assert!(
        keyboard
            .windows(2)
            .all(|pair| pair[0].letter < pair[1].letter)
    );
    assert!(keyboard.contains(&KeyHint {
        letter: 'e',
        mark: Correct
    }));
}

#[test]
fn guesses_wait_for_the_answer() {
    let mut game = WordHunt::new(&Settings::default(), &[PlayerSpec::HUMAN]).unwrap();
    assert_eq!(
        guess(&mut game, "crane"),
        Err(CommandError::Illegal("the word is not chosen yet"))
    );
    game.apply_world(&WorldEvent::AnswerChosen("crane".into()))
        .unwrap();
    assert!(
        game.apply_world(&WorldEvent::AnswerChosen("plumb".into()))
            .is_err()
    );
}

#[test]
fn the_authority_picks_an_answer_from_the_list() {
    let settings = settings(Language::Russian, 6, false);
    let mut authority =
        Authority::<WordHunt>::new(&settings, &[PlayerSpec::HUMAN], 7, Vec::new()).unwrap();
    authority.seal_frame();
    assert_eq!(
        authority.game().view(Viewer::Spectator).phase,
        Phase::Guessing
    );
}

#[test]
fn the_word_of_the_day_is_the_same_for_everyone_and_changes_daily() {
    let daily = |day| {
        let settings = Settings::parse(RawSettings {
            language: Language::Russian,
            mode: PuzzleMode::Daily,
            day,
            ..RawSettings::default()
        })
        .unwrap();
        let game = WordHunt::new(&settings, &[PlayerSpec::HUMAN]).unwrap();
        let mut seeded = mg_core::CoreRng::new(day.into());
        let events = game.roll_world(&mut seeded);
        let mut other = mg_core::CoreRng::new(u64::from(day) + 99);
        assert_eq!(
            events,
            game.roll_world(&mut other),
            "the seed must not matter"
        );
        events
    };
    assert_ne!(daily(20_000), daily(20_001));
    assert_eq!(daily(20_000), daily(20_000));
}

#[test]
fn every_built_in_answer_is_a_valid_word_and_no_list_is_empty() {
    for language in [Language::Russian, Language::English] {
        for length in WordLength::MIN..=WordLength::MAX {
            let length = WordLength::new(length).unwrap();
            let answers = Answers::new(language, length);
            assert!(answers.count() > 300, "{language:?} {length:?}");
            for text in answers.all() {
                assert!(
                    Word::parse(text, language, length).is_ok(),
                    "{language:?}: {text}"
                );
            }
            assert!(answers.daily(DayNumber::constant::<0>()).is_some());
        }
    }
}
