use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Tells a bot thinking elsewhere (see [`crate::BotTurn`]) that its move is no longer wanted:
/// its [`crate::Budget`] runs out at once, so it returns straight away.
#[derive(Clone, Debug, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}
