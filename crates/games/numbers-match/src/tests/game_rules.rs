//! Legacy `game_state.rs` tests (ported) plus game-level additions.

use mg_core::{
    Awaiting, CommandError, CoreRng, Game, GameSettings, OutOfRange, OutcomeKind, PlayerSpec,
    SettingProblem, SettingsError, SetupError, Slot, Status, Viewer,
};

use super::support::{at, board, dealt_game, dealt_game_with_seed, pair, settings, value};
use crate::{
    COLUMNS, CellValue, Command, Hint, HintMode, HintsLeft, INITIAL_CELLS,
    LIMITED_HINTS_PER_REFILL, LIMITED_INITIAL_HINTS, LastAction, LossReason, NumbersMatch, Phase,
    RawSettings, Refills, Refusal, RowIndex, Settings, WorldEvent,
};

const PLAYER: Slot = Slot::new(0);

fn apply(game: &mut NumbersMatch, command: Command) -> Result<(), CommandError<Refusal>> {
    game.apply_command(PLAYER, &command)
}

fn score(game: &NumbersMatch) -> Option<(OutcomeKind, i64)> {
    match game.status() {
        Status::InProgress => None,
        Status::Finished(outcome) => Some((outcome.kind, outcome.scores[0])),
    }
}

/// Rows alternate `121212121` / `343434343`: no two neighbours on any line or in reading order
/// pair up.
fn deal_without_pairs() -> Vec<CellValue> {
    (0..INITIAL_CELLS)
        .map(|index| {
            let (row, column) = (index / COLUMNS, index % COLUMNS);
            value(1 + (column % 2) as u8 + 2 * (row % 2) as u8)
        })
        .collect()
}

// ---- ported legacy tests ----

#[test]
fn test_new_game_has_42_active_cells() {
    let game = dealt_game(HintMode::Limited);
    let view = game.view(Viewer::Player(PLAYER));
    let active = view.cells.iter().filter(|cell| cell.is_active()).count();
    assert_eq!(active, 42);
}

#[test]
fn test_initial_refills_count() {
    assert_eq!(dealt_game(HintMode::Limited).refills_left(), 5);
}

#[test]
fn test_initial_hints_limited_mode() {
    assert_eq!(
        dealt_game(HintMode::Limited).hints_left(),
        HintsLeft::Count(LIMITED_INITIAL_HINTS)
    );
}

#[test]
fn test_initial_hints_unlimited_mode() {
    assert_eq!(
        dealt_game(HintMode::Unlimited).hints_left(),
        HintsLeft::Unlimited
    );
}

#[test]
fn test_initial_hints_disabled_mode() {
    assert_eq!(
        dealt_game(HintMode::Disabled).hints_left(),
        HintsLeft::Count(0)
    );
}

#[test]
fn test_request_hint_disabled_returns_error() {
    let mut game = dealt_game(HintMode::Disabled);
    assert!(apply(&mut game, Command::RequestHint).is_err());
}

#[test]
fn test_refill_decrements_counter() {
    let mut game = dealt_game(HintMode::Limited);
    apply(&mut game, Command::Refill).unwrap();
    assert_eq!(game.refills_left(), 4);
}

#[test]
fn test_refill_adds_hint_in_limited_mode() {
    let mut game = dealt_game(HintMode::Limited);
    apply(&mut game, Command::Refill).unwrap();
    assert_eq!(
        game.hints_left(),
        HintsLeft::Count(LIMITED_INITIAL_HINTS + LIMITED_HINTS_PER_REFILL)
    );
}

#[test]
fn test_refill_fails_when_zero_remaining() {
    let mut game = dealt_game(HintMode::Limited);
    game.set_refills_left(0);
    assert!(apply(&mut game, Command::Refill).is_err());
}

#[test]
fn test_hint_decrements_counter_limited() {
    let mut game = dealt_game(HintMode::Limited);
    apply(&mut game, Command::RequestHint).unwrap();
    assert_eq!(
        game.hints_left(),
        HintsLeft::Count(LIMITED_INITIAL_HINTS - 1)
    );
}

#[test]
fn test_hint_unlimited_never_decrements() {
    let mut game = dealt_game(HintMode::Unlimited);
    apply(&mut game, Command::RequestHint).unwrap();
    assert_eq!(game.hints_left(), HintsLeft::Unlimited);
}

#[test]
fn test_pair_removed_event_generated() {
    let mut game = dealt_game(HintMode::Limited);
    let found = game.find_pair().unwrap();
    apply(&mut game, Command::RemovePair(found)).unwrap();
    assert!(matches!(
        game.last_action(),
        Some(LastAction::PairRemoved { pair, .. }) if *pair == found
    ));
}

/// Legacy drained an event queue; now the next command replaces the last action.
#[test]
fn test_events_cleared_after_take() {
    let mut game = dealt_game(HintMode::Limited);
    let found = game.find_pair().unwrap();
    apply(&mut game, Command::RemovePair(found)).unwrap();
    apply(&mut game, Command::RequestHint).unwrap();
    assert_eq!(game.last_action(), Some(&LastAction::HintShown));
}

#[test]
fn test_fuzz_no_adjacent_horizontal_duplicates() {
    for seed in 0..1000 {
        let game = dealt_game_with_seed(Settings::default(), seed);
        for row in game.cells().chunks(COLUMNS) {
            for neighbours in row.windows(2) {
                let (left, right) = (neighbours[0], neighbours[1]);
                if left.is_active() && right.is_active() {
                    assert_ne!(left, right, "seed {seed}");
                }
            }
        }
    }
}

#[test]
fn invalid_setups_are_rejected() {
    let too_many_refills = RawSettings {
        refills: Refills::MAX + 1,
        ..RawSettings::default()
    };
    assert_eq!(
        Settings::parse(too_many_refills),
        Err(SettingsError {
            field: "refills",
            problem: SettingProblem::OutOfRange(OutOfRange {
                value: 11,
                min: 0,
                max: 10
            }),
        })
    );
    assert!(matches!(
        NumbersMatch::new(&Settings::default(), &[]),
        Err(SetupError::PlayerCount { .. })
    ));
    assert!(matches!(
        NumbersMatch::new(&Settings::default(), &[PlayerSpec::HUMAN; 2]),
        Err(SetupError::PlayerCount { .. })
    ));
    assert!(matches!(
        NumbersMatch::new(&Settings::default(), &[PlayerSpec::BOT]),
        Err(SetupError::InvalidPlayers(_))
    ));
}

#[test]
fn defaults_match_legacy() {
    assert_eq!(
        RawSettings::default(),
        RawSettings {
            hint_mode: HintMode::Limited,
            refills: 5
        }
    );
}

#[test]
fn commands_wait_for_the_deal() {
    let mut game = NumbersMatch::new(&Settings::default(), &[PlayerSpec::HUMAN]).unwrap();
    assert_eq!(game.phase(), Phase::Dealing);
    assert_eq!(game.awaiting(), Awaiting::Turn(PLAYER));
    assert!(matches!(
        apply(&mut game, Command::Refill),
        Err(CommandError::Illegal(Refusal::NotDealtYet))
    ));
    assert_eq!(game.refills_left(), 5);
}

#[test]
fn deal_is_deterministic_and_happens_once() {
    let mut game = dealt_game(HintMode::Limited);
    assert_eq!(game.phase(), Phase::Playing);
    assert_eq!(game.cells(), dealt_game(HintMode::Limited).cells());
    assert_eq!(game.cells().len(), 45);
    assert!(game.roll_world(&mut CoreRng::new(1)).is_empty());
    let again = WorldEvent::BoardDealt {
        values: deal_without_pairs(),
    };
    assert!(game.apply_world(&again).is_err());
}

#[test]
fn malformed_deals_are_rejected() {
    let mut game = NumbersMatch::new(&Settings::default(), &[PlayerSpec::HUMAN]).unwrap();
    let mut short = deal_without_pairs();
    short.pop();
    let mut duplicate_neighbours = deal_without_pairs();
    duplicate_neighbours[1] = duplicate_neighbours[0];
    for values in [short, duplicate_neighbours] {
        let event = WorldEvent::BoardDealt { values };
        assert!(game.apply_world(&event).is_err());
        assert_eq!(game.phase(), Phase::Dealing);
    }
    assert_eq!(CellValue::new(0), None);
    assert_eq!(CellValue::new(10), None);
}

#[test]
fn equal_values_across_a_row_break_are_a_valid_deal() {
    let mut values = deal_without_pairs();
    values[COLUMNS] = values[COLUMNS - 1];
    let mut game = NumbersMatch::new(&Settings::default(), &[PlayerSpec::HUMAN]).unwrap();
    game.apply_world(&WorldEvent::BoardDealt { values })
        .unwrap();
}

#[test]
fn deal_without_pairs_and_refills_is_lost_at_once() {
    let settings = Settings {
        refills: Refills::constant::<0>(),
        ..Settings::default()
    };
    let mut game = NumbersMatch::new(&settings, &[PlayerSpec::HUMAN]).unwrap();
    let values = deal_without_pairs();
    game.apply_world(&WorldEvent::BoardDealt { values })
        .unwrap();
    assert_eq!(game.phase(), Phase::Lost(LossReason::NoMoves));
    assert_eq!(game.awaiting(), Awaiting::Nobody);
}

#[test]
fn removal_reports_deleted_rows_and_clears_the_hint() {
    #[rustfmt::skip]
    let mut game = NumbersMatch::with_board(settings(HintMode::Unlimited), board(&[
        1, 2, 1, 2, 1, 2, 1, 2, 1,
        5, 0, 0, 0, 0, 0, 0, 0, 0,
        5, 3, 0, 0, 0, 0, 0, 0, 0,
    ]));
    apply(&mut game, Command::RequestHint).unwrap();
    let removed = pair(at(2, 0), at(1, 0));
    apply(&mut game, Command::RemovePair(removed)).unwrap();
    assert_eq!(
        game.last_action(),
        Some(&LastAction::PairRemoved {
            pair: removed,
            deleted_rows: vec![RowIndex::new(1)],
        })
    );
    assert_eq!(game.hint(), None);
    assert_eq!(game.cells().len(), 2 * COLUMNS);
    assert_eq!(game.stats().pairs_removed, 1);
}

#[test]
fn refill_reports_appended_cells() {
    let mut game =
        NumbersMatch::with_board(Settings::default(), board(&[1, 2, 0, 0, 0, 0, 0, 0, 0]));
    apply(&mut game, Command::Refill).unwrap();
    assert_eq!(
        game.last_action(),
        Some(&LastAction::Refilled {
            start: at(0, 2),
            values: vec![value(1), value(2)],
        })
    );
    assert_eq!(game.stats().refills_used, 1);
}

#[test]
fn illegal_pairs_change_nothing() {
    let mut game = dealt_game(HintMode::Limited);
    let before = game.state_hash();
    for bad in [
        pair(at(0, 0), at(0, 0)),
        pair(at(0, 0), at(99, 0)),
        pair(at(4, 6), at(4, 7)),
    ] {
        assert!(apply(&mut game, Command::RemovePair(bad)).is_err());
    }
    assert_eq!(game.state_hash(), before);
}

#[test]
fn only_seat_zero_plays() {
    let mut game = dealt_game(HintMode::Limited);
    assert_eq!(
        game.apply_command(Slot::new(1), &Command::Refill),
        Err(CommandError::NotPlaying)
    );
}

#[test]
fn hint_is_the_first_pair_and_stays_until_a_move() {
    let mut game = dealt_game(HintMode::Unlimited);
    let expected = game.board().find_pair_brute_force().unwrap();
    apply(&mut game, Command::RequestHint).unwrap();
    assert_eq!(game.hint(), Some(Hint::Pair(expected)));
    assert_eq!(
        game.view(Viewer::Spectator).hint,
        Some(Hint::Pair(expected))
    );
    assert!(apply(&mut game, Command::RequestHint).is_err());
    assert_eq!(game.stats().hints_used, 1);
    apply(&mut game, Command::Refill).unwrap();
    assert_eq!(game.hint(), None);
    apply(&mut game, Command::RequestHint).unwrap();
}

#[test]
fn hint_suggests_refill_when_stuck() {
    let mut game = NumbersMatch::with_board(
        settings(HintMode::Limited),
        board(&[1, 2, 3, 0, 0, 0, 0, 0, 0]),
    );
    apply(&mut game, Command::RequestHint).unwrap();
    assert_eq!(game.hint(), Some(Hint::SuggestRefill));
}

#[test]
fn limited_hints_run_out() {
    let mut game = dealt_game(HintMode::Limited);
    for _ in 0..LIMITED_INITIAL_HINTS {
        apply(&mut game, Command::RequestHint).unwrap();
        let hinted = game.find_pair().unwrap();
        apply(&mut game, Command::RemovePair(hinted)).unwrap();
    }
    assert_eq!(game.hints_left(), HintsLeft::Count(0));
    assert!(apply(&mut game, Command::RequestHint).is_err());
}

#[test]
fn clearing_the_board_wins_with_score_one() {
    let mut game =
        NumbersMatch::with_board(Settings::default(), board(&[3, 0, 0, 0, 0, 0, 0, 0, 0, 7]));
    apply(&mut game, Command::RemovePair(pair(at(0, 0), at(1, 0)))).unwrap();
    assert_eq!(game.phase(), Phase::Won);
    assert_eq!(score(&game), Some((OutcomeKind::Victory, 1)));
    assert!(game.cells().is_empty());
    assert_eq!(
        apply(&mut game, Command::Refill),
        Err(CommandError::GameOver)
    );
}

#[test]
fn no_pair_and_no_refill_loses_with_score_zero() {
    let settings = Settings {
        refills: Refills::constant::<0>(),
        ..Settings::default()
    };
    let mut game = NumbersMatch::with_board(settings, board(&[1, 2, 5, 5, 0, 0, 0, 0, 0]));
    apply(&mut game, Command::RemovePair(pair(at(0, 2), at(0, 3)))).unwrap();
    assert_eq!(game.phase(), Phase::Lost(LossReason::NoMoves));
    assert_eq!(score(&game), Some((OutcomeKind::Defeat, 0)));
}

#[test]
fn no_pair_with_a_refill_left_keeps_playing() {
    let settings = Settings {
        refills: Refills::constant::<1>(),
        ..Settings::default()
    };
    let mut game = NumbersMatch::with_board(settings, board(&[1, 2, 5, 5, 0, 0, 0, 0, 0]));
    apply(&mut game, Command::RemovePair(pair(at(0, 2), at(0, 3)))).unwrap();
    assert_eq!(game.phase(), Phase::Playing);
    // 1 2 _ _ 1 2: still no pair and no refills left.
    apply(&mut game, Command::Refill).unwrap();
    assert_eq!(game.phase(), Phase::Lost(LossReason::NoMoves));
}

#[test]
fn dropping_the_player_loses() {
    for mut game in [
        dealt_game(HintMode::Limited),
        NumbersMatch::new(&Settings::default(), &[PlayerSpec::HUMAN]).unwrap(),
    ] {
        game.drop_player(PLAYER);
        assert_eq!(game.phase(), Phase::Lost(LossReason::Dropped));
        assert_eq!(score(&game), Some((OutcomeKind::Defeat, 0)));
        assert_eq!(game.awaiting(), Awaiting::Nobody);
        assert!(game.roll_world(&mut CoreRng::new(1)).is_empty());
    }
}

#[test]
fn greedy_play_always_finishes() {
    let mut victories = 0;
    for seed in 0..50 {
        let mut game = dealt_game_with_seed(Settings::default(), seed);
        while !game.phase().is_over() {
            let command = game
                .find_pair()
                .map_or(Command::Refill, Command::RemovePair);
            apply(&mut game, command).unwrap();
        }
        victories += usize::from(game.phase() == Phase::Won);
    }
    assert!(victories < 50, "{victories}");
}
