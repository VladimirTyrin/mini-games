use mg_core::CancelToken;

/// Cancels its token when dropped: whatever way its owner goes away.
#[derive(Debug)]
pub(crate) struct CancelOnDrop(CancelToken);

impl CancelOnDrop {
    pub(crate) fn new(token: CancelToken) -> Self {
        Self(token)
    }

    pub(crate) fn token(&self) -> &CancelToken {
        &self.0
    }
}

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
