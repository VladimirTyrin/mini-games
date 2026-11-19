use mg_core::EnglishText;

use super::*;

/// Every wire value of the game's refusal enum decodes to a variant that encodes back to it, and
/// reads as that variant's English.
fn check_game<R: WireRefusal>(is_wire_value: fn(i32) -> bool, case: fn(i32) -> Reason) {
    let mut checked = 0;
    for value in (1..=255).filter(|value| is_wire_value(*value)) {
        let refusal = R::refusal_from_wire(value)
            .unwrap_or_else(|| panic!("wire value {value} has no variant"));
        assert_eq!(refusal.refusal_to_wire(), case(value));
        let english = refusal_english(&proto::Refusal {
            reason: Some(case(value)),
        });
        assert!(!english.is_empty(), "wire value {value} has no English");
        assert_eq!(english, refusal.english());
        checked += 1;
    }
    assert!(checked > 0, "no wire values");
    assert!(R::refusal_from_wire(0).is_none());
}

#[test]
fn every_games_wire_refusal_reads_as_its_english_and_round_trips() {
    check_game::<mg_snake::Refusal>(
        |value| proto::SnakeRefusal::try_from(value).is_ok(),
        Reason::Snake,
    );
    check_game::<mg_tictactoe::Refusal>(
        |value| proto::TicTacToeRefusal::try_from(value).is_ok(),
        Reason::Tictactoe,
    );
    check_game::<mg_puzzle2048::Refusal>(
        |value| proto::Puzzle2048Refusal::try_from(value).is_ok(),
        Reason::Puzzle2048,
    );
    check_game::<mg_numbers_match::Refusal>(
        |value| proto::NumbersMatchRefusal::try_from(value).is_ok(),
        Reason::NumbersMatch,
    );
    check_game::<mg_word_hunt::Refusal>(
        |value| proto::WordHuntRefusal::try_from(value).is_ok(),
        Reason::WordHunt,
    );
    check_game::<mg_checkers::Refusal>(
        |value| proto::CheckersRefusal::try_from(value).is_ok(),
        Reason::Checkers,
    );
    check_game::<mg_sea_battle::Refusal>(
        |value| proto::SeaBattleRefusal::try_from(value).is_ok(),
        Reason::SeaBattle,
    );
    // xtask:check_game
}

#[test]
fn common_reasons_read_as_the_cores_english_and_unknown_ones_generically() {
    let common = |value: i32| {
        refusal_english(&proto::Refusal {
            reason: Some(Reason::Common(value)),
        })
    };
    assert_eq!(
        common(proto::CommonRefusal::NotYourTurn.into()),
        CommandError::<Infallible>::NotYourTurn.english()
    );
    for value in (1..=255).filter(|value| proto::CommonRefusal::try_from(*value).is_ok()) {
        assert_ne!(common(value), UNKNOWN, "common value {value}");
    }
    assert_eq!(common(0), UNKNOWN);
    assert_eq!(common(999), UNKNOWN);
    assert_eq!(refusal_english(&proto::Refusal { reason: None }), UNKNOWN);
    assert_eq!(
        refusal_english(&proto::Refusal {
            reason: Some(Reason::WordHunt(999)),
        }),
        UNKNOWN
    );
}

#[test]
fn a_games_malformed_input_reads_as_malformed() {
    let error = CommandError::Illegal(mg_puzzle2048::Refusal::SpawnCellOccupied);
    assert_eq!(refusal_english(&crate::refusal_to_wire(&error)), MALFORMED);
}

#[test]
fn setting_rules_read_as_the_cores_english() {
    assert_eq!(
        setting_rule_english(proto::SettingRule::PowerOfTwo.into()),
        SettingRule::PowerOfTwo.english()
    );
    for value in (1..=255).filter(|value| proto::SettingRule::try_from(*value).is_ok()) {
        assert_ne!(setting_rule_english(value), UNKNOWN_RULE, "rule {value}");
    }
    assert_eq!(setting_rule_english(0), UNKNOWN_RULE);
}
