use std::fmt;

use crate::out_of_range::OutOfRange;

/// What is wrong with one setting's value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingProblem {
    OutOfRange(OutOfRange),
    /// A rule a range cannot express; `rule` reads as a sentence ("must be a power of two").
    RuleBroken {
        rule: &'static str,
        value: u32,
    },
}

impl From<OutOfRange> for SettingProblem {
    fn from(error: OutOfRange) -> Self {
        Self::OutOfRange(error)
    }
}

impl fmt::Display for SettingProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange(error) => error.fmt(f),
            Self::RuleBroken { rule, value } => write!(f, "{rule}, got {value}"),
        }
    }
}

impl std::error::Error for SettingProblem {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OutOfRange(error) => Some(error),
            Self::RuleBroken { .. } => None,
        }
    }
}
