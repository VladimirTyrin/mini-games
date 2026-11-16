/// Wire data that cannot be turned into a valid domain value. Wire input is untrusted: every
/// conversion from the wire must return this instead of panicking or clamping silently.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum WireError {
    #[error("missing field `{0}`")]
    Missing(&'static str),
    #[error("message belongs to another game")]
    WrongGame,
    #[error("field `{0}` is out of range")]
    OutOfRange(&'static str),
    #[error("field `{field}` is out of range")]
    IntegerOutOfRange {
        field: &'static str,
        source: std::num::TryFromIntError,
    },
    #[error("unknown enum value in `{0}`")]
    UnknownEnumValue(&'static str),
}
