use std::fmt::Debug;

use crate::game::Game;

/// What only the authority knows of a game (see [`Game::Secret`]), and how seats take part in it.
pub trait GameSecret: Default + Clone + Debug + Send + 'static {
    /// A seat's private command (its fleet): it reaches only the authority, never a frame.
    type Command: Clone + Debug + PartialEq + Send + Sync + 'static;
    /// What one seat may see of the secret (its own fleet), sent to that seat alone.
    type SeatView: Clone + Debug + PartialEq + Send + Sync + 'static;
}

/// Games that hide nothing.
impl GameSecret for () {
    type Command = ();
    type SeatView = ();
}

pub type SecretCommand<G> = <<G as Game>::Secret as GameSecret>::Command;
pub type SeatSecret<G> = <<G as Game>::Secret as GameSecret>::SeatView;
