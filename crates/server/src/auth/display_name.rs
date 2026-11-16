use std::fmt;

use super::auth_error::AuthError;

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
        if trimmed
            .chars()
            .any(|c| c.is_control() || is_invisible_format(c))
        {
            return Err(AuthError::InvalidDisplayName(
                "contains invisible characters",
            ));
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// For names from trusted sources (Google profile, bot config) that may exceed our limits.
    pub(crate) fn sanitize(raw: &str, fallback: &str) -> Self {
        let cleaned: String = raw
            .trim()
            .chars()
            .filter(|c| !c.is_control() && !is_invisible_format(*c))
            .take(Self::MAX_CHARS)
            .collect();
        match cleaned.trim() {
            "" => Self(fallback.to_owned()),
            name => Self(name.to_owned()),
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Case-insensitive key used to keep online names unique.
    pub(crate) fn uniqueness_key(&self) -> String {
        self.0.to_lowercase()
    }
}

fn is_invisible_format(c: char) -> bool {
    matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}' | '\u{FEFF}')
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
    }
}
