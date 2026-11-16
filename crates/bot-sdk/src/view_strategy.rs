use mg_core::Slot;
use mg_protocol::proto;

/// The simplest strategy: look at the rendered view (what a human sees) and answer with a
/// command. Wrap it in [`crate::ViewBot`] to play.
pub trait ViewStrategy {
    /// `view` is rendered for `me`. Called once per new state, whoever is to move; return `None`
    /// to pass.
    fn decide(&mut self, view: &proto::GameView, me: Slot) -> Option<proto::GameCommand>;
}
