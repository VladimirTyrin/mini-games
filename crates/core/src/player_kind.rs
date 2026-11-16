#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerKind {
    Human,
    /// Built-in bot executed by the match authority.
    Bot,
}
