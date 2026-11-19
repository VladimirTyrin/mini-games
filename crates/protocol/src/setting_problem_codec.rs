use mg_core::{SettingProblem, SettingRule, SettingsError};

use crate::proto;
use crate::proto::setting_problem::{BrokenRule, OutOfRange, Problem};

pub fn setting_problem_to_wire(error: &SettingsError) -> proto::SettingProblem {
    let problem = match error.problem {
        SettingProblem::OutOfRange(range) => Problem::OutOfRange(OutOfRange {
            value: range.value,
            min: range.min,
            max: range.max,
        }),
        SettingProblem::RuleBroken { rule, value } => Problem::BrokenRule(BrokenRule {
            rule: setting_rule_to_wire(rule).into(),
            value,
        }),
    };
    proto::SettingProblem {
        field: error.field.to_owned(),
        problem: Some(problem),
    }
}

pub(crate) fn setting_rule_from_wire(rule: i32) -> Option<SettingRule> {
    Some(match proto::SettingRule::try_from(rule).ok()? {
        proto::SettingRule::Unspecified => return None,
        proto::SettingRule::PowerOfTwo => SettingRule::PowerOfTwo,
        proto::SettingRule::TooLargeForBoard => SettingRule::TooLargeForBoard,
        proto::SettingRule::FixedForDailyPuzzle => SettingRule::FixedForDailyPuzzle,
    })
}

fn setting_rule_to_wire(rule: SettingRule) -> proto::SettingRule {
    match rule {
        SettingRule::PowerOfTwo => proto::SettingRule::PowerOfTwo,
        SettingRule::TooLargeForBoard => proto::SettingRule::TooLargeForBoard,
        SettingRule::FixedForDailyPuzzle => proto::SettingRule::FixedForDailyPuzzle,
    }
}
