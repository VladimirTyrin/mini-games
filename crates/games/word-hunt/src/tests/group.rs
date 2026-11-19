//! Group games: 2–4 seats take turns on one board; whoever finds the word wins.

use mg_core::{
    Authority, BotSeat, CommandError, Game, GameSettings, HasBots, OutcomeKind, PlayerSpec,
    SetupError, Slot, Status, Viewer,
};

use super::table::Table;
use crate::model::Answers;
use crate::{
    BotKind, Command, FirstTurn, GROUP_GUESS_CAP, Language, Phase, PuzzleMode, RawSettings,
    Refusal, Settings, WordHunt, WorldEvent,
};

const HUMAN: PlayerSpec = PlayerSpec::HUMAN;
const BOT: PlayerSpec = PlayerSpec::BOT;
const FIRST: Slot = Slot::new(0);
const SECOND: Slot = Slot::new(1);
const THIRD: Slot = Slot::new(2);

fn english() -> Settings {
    Settings::parse(RawSettings::default()).unwrap()
}

/// A started group game: answer `crane`, `first` to guess.
fn group(players: &[PlayerSpec], first: Slot) -> Table {
    let mut game = Table::new(WordHunt::new(&english(), players).unwrap(), "crane");
    game.world(&WorldEvent::FirstPlayerChosen(first)).unwrap();
    game
}

fn guess(game: &mut Table, slot: Slot, text: &str) -> Result<(), CommandError<Refusal>> {
    game.command(slot, &Command::Guess(text.into()))
}

fn outcome(game: &WordHunt) -> mg_core::Outcome {
    match game.status() {
        Status::Finished(outcome) => outcome,
        Status::InProgress => panic!("still running"),
    }
}

#[test]
fn players_take_turns_on_one_board() {
    let mut game = group(&[HUMAN, HUMAN, HUMAN], SECOND);
    assert_eq!(
        guess(&mut game, FIRST, "plumb"),
        Err(CommandError::NotYourTurn)
    );
    guess(&mut game, SECOND, "plumb").unwrap();
    guess(&mut game, THIRD, "fight").unwrap();
    guess(&mut game, FIRST, "dozen").unwrap();
    let view = game.view(Viewer::Player(FIRST));
    assert_eq!(view.turn, Some(SECOND));
    let authors: Vec<Slot> = view.guesses.iter().map(|guess| guess.by).collect();
    assert_eq!(authors, [SECOND, THIRD, FIRST]);
}

#[test]
fn whoever_finds_the_word_wins() {
    let mut game = group(&[HUMAN, HUMAN], FIRST);
    guess(&mut game, FIRST, "plumb").unwrap();
    guess(&mut game, SECOND, "crane").unwrap();
    let result = outcome(&game);
    assert_eq!(result.kind, OutcomeKind::Winner);
    assert_eq!(result.winners, [SECOND]);
    assert_eq!(result.scores, [0, 1]);
    assert_eq!(game.view(Viewer::Spectator).winner, Some(SECOND));
}

#[test]
fn a_group_is_not_held_to_the_solo_guess_limit_but_draws_at_the_cap() {
    let mut game = group(&[HUMAN, HUMAN], FIRST);
    let settings = english();
    let words = Answers::new(settings.language, settings.word_length())
        .all()
        .filter(|word| *word != "crane");
    let mut slot = FIRST;
    for word in words.take(GROUP_GUESS_CAP) {
        guess(&mut game, slot, word).unwrap();
        slot = if slot == FIRST { SECOND } else { FIRST };
    }
    let view = game.view(Viewer::Spectator);
    assert_eq!(view.phase, Phase::Failed);
    assert_eq!(view.answer.as_deref(), Some("crane"));
    assert_eq!(outcome(&game).kind, OutcomeKind::Draw);
}

#[test]
fn the_last_one_at_the_table_wins_when_the_others_leave() {
    let mut game = group(&[HUMAN, HUMAN, HUMAN], FIRST);
    game.drop_player(FIRST);
    // The leaver's turn passes on.
    assert_eq!(game.view(Viewer::Spectator).turn, Some(SECOND));
    game.drop_player(THIRD);
    assert_eq!(game.view(Viewer::Spectator).phase, Phase::Forfeited);
    assert_eq!(outcome(&game).winners, [SECOND]);
}

#[test]
fn bots_alone_do_not_play_on() {
    let mut game = group(&[HUMAN, BOT, BOT], FIRST);
    game.drop_player(FIRST);
    assert_eq!(game.view(Viewer::Spectator).phase, Phase::Abandoned);
    assert_eq!(outcome(&game).kind, OutcomeKind::Draw);
}

#[test]
fn the_word_of_the_day_is_played_alone() {
    let daily = Settings::parse(RawSettings {
        mode: PuzzleMode::Daily,
        day: 20_000,
        ..RawSettings::default()
    })
    .unwrap();
    assert!(matches!(
        WordHunt::new(&daily, &[HUMAN, BOT]),
        Err(SetupError::InvalidPlayers(_))
    ));
}

fn bot_seat(slot: Slot, kind: BotKind) -> BotSeat<WordHunt> {
    BotSeat {
        slot,
        bot: WordHunt::make_bot(kind),
        budget_nodes: WordHunt::default_bot_budget(kind),
    }
}

/// Plays a person and a bot out; the person's guesses come from `person` (a bot too).
fn play_out(settings: &Settings, kind: BotKind, person: BotKind) -> Authority<WordHunt> {
    let mut authority =
        Authority::<WordHunt>::new(settings, &[HUMAN, BOT], 11, vec![bot_seat(SECOND, kind)])
            .unwrap();
    let mut stand_in = WordHunt::make_bot(person);
    let mut rng = mg_core::CoreRng::new(5);
    authority.seal_frame();
    for _ in 0..=GROUP_GUESS_CAP {
        if authority.game().status().is_finished() {
            break;
        }
        if authority.game().awaiting() == mg_core::Awaiting::Turn(FIRST) {
            let mut budget = mg_core::Budget::new(WordHunt::default_bot_budget(person));
            let command = stand_in
                .decide(authority.game(), FIRST, &mut rng, &mut budget)
                .unwrap();
            authority.submit(FIRST, command).unwrap();
        }
        authority.run_bots();
        authority.seal_frame();
    }
    authority
}

/// Every level, with and without hard mode, only ever makes legal guesses, and the game ends
/// with the word found well before the cap.
#[test]
fn bots_play_legal_guesses_to_the_end() {
    for kind in [BotKind::Easy, BotKind::Medium, BotKind::Hard] {
        for hard_mode in [false, true] {
            let settings = Settings::parse(RawSettings {
                language: Language::Russian,
                hard_mode,
                ..RawSettings::default()
            })
            .unwrap();
            let authority = play_out(&settings, kind, kind);
            let view = authority.game().view(Viewer::Spectator);
            assert_eq!(
                view.phase,
                Phase::Solved,
                "{kind:?} hard mode {hard_mode}: {view:?}"
            );
            assert_eq!(
                authority.rejected_bot_commands(),
                0,
                "{kind:?} hard mode {hard_mode}"
            );
        }
    }
}

/// A bot never peeks: its first guess cannot be the answer more often than chance allows. Over
/// many seeds the hard bot's opening guess almost never hits.
#[test]
fn a_bot_guesses_from_the_hints_only() {
    let settings = english();
    let mut hits = 0;
    for seed in 0..40 {
        let bots = vec![bot_seat(FIRST, BotKind::Hard)];
        let mut authority =
            Authority::<WordHunt>::new(&settings, &[BOT, HUMAN], seed, bots).unwrap();
        // Answer and first player come in the first frame; the bot may move in a later one.
        for _ in 0..3 {
            authority.run_bots();
            authority.seal_frame();
        }
        let view = authority.game().view(Viewer::Spectator);
        if view.guesses.first().is_some_and(|first| {
            first
                .marks
                .iter()
                .all(|mark| *mark == crate::LetterMark::Correct)
        }) {
            hits += 1;
        }
    }
    assert!(hits <= 1, "{hits} opening hits in 40 games");
}

#[test]
fn a_player_may_give_up_out_of_turn_and_the_others_play_on() {
    let mut game = group(&[HUMAN, HUMAN, HUMAN], FIRST);
    game.command(THIRD, &Command::GiveUp).unwrap();
    guess(&mut game, FIRST, "plumb").unwrap();
    // The turn skips the seat that gave up.
    assert_eq!(game.view(Viewer::Spectator).turn, Some(SECOND));
    assert_eq!(
        game.command(THIRD, &Command::GiveUp),
        Err(CommandError::Illegal(Refusal::CannotGiveUp))
    );
}

#[test]
fn a_player_who_gave_up_has_left_play_while_the_others_have_not() {
    let mut game = group(&[HUMAN, HUMAN, HUMAN], FIRST);
    game.command(THIRD, &Command::GiveUp).unwrap();
    assert!(game.has_left_play(THIRD));
    assert!(!game.has_left_play(FIRST));
}

#[test]
fn giving_up_against_a_bot_lets_the_bot_win() {
    let mut game = group(&[HUMAN, BOT], FIRST);
    game.command(FIRST, &Command::GiveUp).unwrap();
    assert_eq!(game.view(Viewer::Spectator).phase, Phase::Forfeited);
    assert_eq!(outcome(&game).winners, [SECOND]);
}

#[test]
fn giving_up_on_ones_own_turn_passes_the_turn() {
    let mut game = group(&[HUMAN, HUMAN, HUMAN], SECOND);
    game.command(SECOND, &Command::GiveUp).unwrap();
    let view = game.view(Viewer::Spectator);
    assert_eq!(view.turn, Some(THIRD));
    assert_eq!(view.playing, [FIRST, THIRD]);
}

#[test]
fn the_last_person_giving_up_among_bots_ends_the_game() {
    let mut game = group(&[HUMAN, BOT, BOT], FIRST);
    game.command(FIRST, &Command::GiveUp).unwrap();
    assert_eq!(game.view(Viewer::Spectator).phase, Phase::Abandoned);
}

#[test]
fn the_first_turn_follows_the_setting() {
    let first_of = |first_turn| {
        let settings = Settings::parse(RawSettings {
            first_turn,
            ..RawSettings::default()
        })
        .unwrap();
        let mut game = WordHunt::new(&settings, &[HUMAN, BOT, BOT]).unwrap();
        game.apply_world(&WorldEvent::WordChosen).unwrap();
        game.roll_world(&mut mg_core::CoreRng::new(3))
            .into_iter()
            .find_map(|event| match event {
                WorldEvent::FirstPlayerChosen(slot) => Some(slot),
                _ => None,
            })
    };
    assert_eq!(first_of(FirstTurn::HostFirst), Some(FIRST));
    assert_eq!(first_of(FirstTurn::HostLast), Some(SECOND));
}

#[test]
fn with_host_last_the_host_closes_every_round() {
    let settings = Settings::parse(RawSettings {
        first_turn: FirstTurn::HostLast,
        ..RawSettings::default()
    })
    .unwrap();
    let mut game = Table::new(
        WordHunt::new(&settings, &[HUMAN, BOT, BOT]).unwrap(),
        "crane",
    );
    game.roll();
    for word in ["plumb", "fight", "dozen", "quick", "jumpy", "vowel"] {
        let turn = game.view(Viewer::Player(FIRST)).turn.unwrap();
        guess(&mut game, turn, word).unwrap();
    }
    let authors: Vec<Slot> = game
        .view(Viewer::Player(FIRST))
        .guesses
        .iter()
        .map(|guess| guess.by)
        .collect();
    assert_eq!(authors, [SECOND, THIRD, FIRST, SECOND, THIRD, FIRST]);
}

#[test]
fn a_first_player_other_than_the_settings_name_is_refused() {
    let settings = Settings::parse(RawSettings {
        first_turn: FirstTurn::HostFirst,
        ..RawSettings::default()
    })
    .unwrap();
    let mut game = WordHunt::new(&settings, &[HUMAN, BOT]).unwrap();
    game.apply_world(&WorldEvent::WordChosen).unwrap();
    assert_eq!(
        game.apply_world(&WorldEvent::FirstPlayerChosen(SECOND)),
        Err(CommandError::Illegal(Refusal::FirstPlayerAgainstSettings))
    );
    game.apply_world(&WorldEvent::FirstPlayerChosen(FIRST))
        .unwrap();
}

#[test]
fn a_table_of_bots_plays_on_when_one_is_dropped() {
    let mut game = group(&[BOT, BOT, BOT], FIRST);
    game.drop_player(SECOND);
    assert_eq!(game.status(), Status::InProgress);
    game.drop_player(THIRD);
    let result = outcome(&game);
    assert_eq!(result.kind, OutcomeKind::Winner);
    assert_eq!(result.winners, [FIRST]);
}

#[test]
fn a_lone_bot_dropped_ends_the_game() {
    let mut game = Table::new(WordHunt::new(&english(), &[BOT]).unwrap(), "crane");
    game.drop_player(FIRST);
    assert!(game.status().is_finished(), "a game nobody plays must end");
}
