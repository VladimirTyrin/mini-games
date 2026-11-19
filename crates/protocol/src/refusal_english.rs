use std::convert::Infallible;

use mg_core::{CommandError, EnglishText, SettingRule};

use crate::proto;
use crate::proto::refusal::Reason;
use crate::setting_problem_codec::setting_rule_from_wire;
use crate::wire_refusal::WireRefusal;

/// What the wire tells of input no honest client sends; the server's `ErrorResponse.message`
/// names the detail.
const MALFORMED: &str = "malformed input";
const PAUSED: &str = "the match is paused";
/// A reason this build does not know (a newer server), or none at all.
const UNKNOWN: &str = "the command was refused";

/// The English for a refusal received on the wire, from the same words as the core's
/// [`EnglishText`]: for logs and bot clients (the server's `ErrorResponse.message` is this text,
/// except for malformed input, whose detail it names).
/// The app words refusals in the player's language and never uses it.
pub fn refusal_english(refusal: &proto::Refusal) -> &'static str {
    let english = match refusal.reason {
        None => None,
        Some(Reason::Common(value)) => common_english(value),
        Some(Reason::Snake(value)) => game_english::<mg_snake::Refusal>(value),
        Some(Reason::Tictactoe(value)) => game_english::<mg_tictactoe::Refusal>(value),
        Some(Reason::Puzzle2048(value)) => game_english::<mg_puzzle2048::Refusal>(value),
        Some(Reason::NumbersMatch(value)) => game_english::<mg_numbers_match::Refusal>(value),
        Some(Reason::WordHunt(value)) => game_english::<mg_word_hunt::Refusal>(value),
        Some(Reason::Checkers(value)) => game_english::<mg_checkers::Refusal>(value),
        Some(Reason::SeaBattle(value)) => game_english::<mg_sea_battle::Refusal>(value),
        // xtask:refusal_english
    };
    english.unwrap_or(UNKNOWN)
}

/// The English for a `SettingRule` received on the wire ("must be a power of two").
pub fn setting_rule_english(rule: i32) -> &'static str {
    setting_rule_from_wire(rule).map_or(UNKNOWN_RULE, SettingRule::english)
}

const UNKNOWN_RULE: &str = "breaks a rule of the game";

fn game_english<R: WireRefusal>(value: i32) -> Option<&'static str> {
    R::refusal_from_wire(value).map(R::english)
}

fn common_english(value: i32) -> Option<&'static str> {
    type Common = CommandError<Infallible>;
    Some(match proto::CommonRefusal::try_from(value).ok()? {
        proto::CommonRefusal::Unspecified => return None,
        proto::CommonRefusal::GameOver => Common::GameOver.english(),
        proto::CommonRefusal::NotYourTurn => Common::NotYourTurn.english(),
        proto::CommonRefusal::NotPlaying => Common::NotPlaying.english(),
        proto::CommonRefusal::Busy => Common::Busy.english(),
        proto::CommonRefusal::Malformed => MALFORMED,
        proto::CommonRefusal::Paused => PAUSED,
    })
}

#[cfg(test)]
#[path = "refusal_english_tests.rs"]
mod tests;
