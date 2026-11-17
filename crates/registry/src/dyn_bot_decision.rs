use std::any::Any;

/// [`mg_core::DecidedBotTurn`] with the game type erased; only the authority that began the turn
/// can take it back.
#[derive(Debug)]
pub struct DynBotDecision(Box<dyn Any + Send>);

impl DynBotDecision {
    pub fn new(decided: impl Any + Send) -> Self {
        Self(Box::new(decided))
    }

    pub(crate) fn downcast<T: Any>(self) -> Option<T> {
        self.0.downcast::<T>().ok().map(|decided| *decided)
    }
}
