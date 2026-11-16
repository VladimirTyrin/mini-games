use std::fmt;

use super::auth_error::AuthError;
use crate::text;

/// Name shown to other players. Validated: trimmed, 1..=MAX_CHARS characters, no control or
/// invisible formatting characters (they allow spoofing other players' names).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DisplayName(String);

impl DisplayName {
    pub(crate) const MAX_CHARS: usize = 24;

    pub(crate) fn parse(raw: &str) -> Result<Self, AuthError> {
        let trimmed = raw.trim();
        let length = trimmed.chars().count();
        if length == 0 || length > Self::MAX_CHARS {
            return Err(AuthError::InvalidDisplayName("must be 1 to 24 characters"));
        }
        if trimmed.chars().any(text::is_hidden) {
            return Err(AuthError::InvalidDisplayName(
                "contains invisible characters",
            ));
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// For names from trusted sources (Google profile, bot config) that may exceed our limits.
    pub(crate) fn sanitize(raw: &str, fallback: &str) -> Self {
        match text::clean(raw, Self::MAX_CHARS) {
            name if name.is_empty() => Self(fallback.to_owned()),
            name => Self(name),
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DisplayName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_and_trims_normal_names() {
        assert_eq!(DisplayName::parse("  Вова ").unwrap().as_str(), "Вова");
    }

    #[test]
    fn rejects_empty_long_and_spoofing_names() {
        assert!(DisplayName::parse("   ").is_err());
        assert!(DisplayName::parse(&"x".repeat(25)).is_err());
        assert!(DisplayName::parse("ad\u{202E}min").is_err());
        assert!(DisplayName::parse("a\nb").is_err());
        assert!(DisplayName::parse("ad\u{2067}min").is_err());
        assert!(DisplayName::parse("\u{3164}").is_err());
        assert!(DisplayName::parse("a\u{00AD}b").is_err());
        assert!(DisplayName::parse("a\u{061C}b").is_err());
    }

    #[test]
    fn sanitize_truncates_and_falls_back() {
        assert_eq!(
            DisplayName::sanitize(&"y".repeat(40), "x")
                .as_str()
                .chars()
                .count(),
            24
        );
        assert_eq!(
            DisplayName::sanitize("\u{200B}", "Player").as_str(),
            "Player"
        );
        assert_eq!(DisplayName::sanitize("A\u{2066}da", "x").as_str(), "Ada");
    }
}
