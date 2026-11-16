use axum::extract::ws::Utf8Bytes;
use mg_core::Tick;
use mg_protocol::proto;

/// A replay the lobby watches together, kept so late joiners and resumed members get the file
/// and the current playback controls.
#[derive(Debug)]
pub(super) struct SharedReplay {
    /// Encoded `SharedReplay` push (the file as base64), built once off the hub task.
    pub push: Utf8Bytes,
    pub end_tick: Tick,
    pub host_only_control: bool,
    /// Last `SET_SPEED`.
    pub speed: Option<proto::ReplayControl>,
    /// Last `PLAY`, `PAUSE` or `SEEK`.
    pub playback: Option<proto::ReplayControl>,
}
