use std::convert::Infallible;

/// The English words for a reason (a refusal, a setting rule): the one text logs, bot clients
/// and [`std::fmt::Display`] use. Players never see it; clients word the wire value themselves.
pub trait EnglishText: Copy {
    fn english(self) -> &'static str;
}

/// For [`crate::CommandError`]'s common reasons, which name no game's refusal.
impl EnglishText for Infallible {
    fn english(self) -> &'static str {
        match self {}
    }
}
