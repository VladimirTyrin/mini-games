use axum::extract::ws::Utf8Bytes;
use mg_core::Tick;

/// An uploaded replay that every viewer can play.
#[derive(Debug)]
pub(crate) struct ReplayUpload {
    /// Encoded `SharedReplay` push.
    pub push: Utf8Bytes,
    pub end_tick: Tick,
}
