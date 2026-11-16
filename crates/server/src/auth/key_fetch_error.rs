#[derive(Debug, thiserror::Error)]
pub(crate) enum KeyFetchError {
    #[error("request failed: {0}")]
    Request(#[source] ureq::Error),
    #[error("unreadable key set: {0}")]
    Body(#[source] ureq::Error),
}
