use mg_registry::ReplayError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ReplayCheckError {
    #[error(transparent)]
    Invalid(#[from] ReplayError),
    #[error("the replay could not be checked")]
    TaskFailed,
}
