//! Cleaning of user-supplied text shown to other players (names, lobby names, chat).

/// Characters that render as nothing (or reorder their neighbours) and so let one player
/// imitate another's name or hide text: zero-width and bidi controls, the soft hyphen and the
/// Hangul fillers that render as blanks.
pub(crate) fn is_hidden(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{00AD}'
                | '\u{061C}'
                | '\u{115F}'
                | '\u{1160}'
                | '\u{180E}'
                | '\u{200B}'..='\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2060}'..='\u{2064}'
                | '\u{2066}'..='\u{2069}'
                | '\u{3164}'
                | '\u{FEFF}'
                | '\u{FFA0}'
        )
}

/// Trims, drops hidden characters and keeps at most `max_chars`.
pub(crate) fn clean(raw: &str, max_chars: usize) -> String {
    let visible: String = raw.chars().filter(|c| !is_hidden(*c)).collect();
    let limited: String = visible.trim().chars().take(max_chars).collect();
    limited.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_bidi_isolates_fillers_and_soft_hyphens() {
        for hidden in [
            '\u{2066}', '\u{2069}', '\u{061C}', '\u{00AD}', '\u{115F}', '\u{3164}', '\u{202E}',
        ] {
            assert!(is_hidden(hidden), "{:?}", hidden);
        }
        assert!(!is_hidden('a'));
        assert!(!is_hidden('Ж'));
    }

    #[test]
    fn clean_strips_hidden_characters_and_limits_length() {
        assert_eq!(clean(" a\u{2067}b\u{3164} ", 10), "ab");
        assert_eq!(clean("\u{00AD} x", 10), "x");
        assert_eq!(clean("abcdef", 3), "abc");
    }
}
