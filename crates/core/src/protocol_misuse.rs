use std::fmt;

use crate::english_text::EnglishText;

/// A host or client using the core in a way no honest one does; the core's own counterpart of a
/// game's malformed-input refusals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolMisuse {
    /// A private command (or its default) for a seat that owes none.
    NoSecretOwed,
    /// A default private command asked of a game that has none.
    NoDefaultSecret,
    /// A private command sent to a game that takes none.
    NoSecretCommands,
}

impl EnglishText for ProtocolMisuse {
    fn english(self) -> &'static str {
        match self {
            Self::NoSecretOwed => "this seat owes no private command",
            Self::NoDefaultSecret => "this game chooses no private command",
            Self::NoSecretCommands => "this game takes no private commands",
        }
    }
}

impl fmt::Display for ProtocolMisuse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.english())
    }
}
