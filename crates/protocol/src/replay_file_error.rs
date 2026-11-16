use crate::wire_error::WireError;

#[derive(Debug, thiserror::Error)]
pub enum ReplayFileError {
    #[error("not a replay file: {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("replay format {found} is not supported (expected {supported})")]
    UnsupportedFormat { found: u32, supported: u32 },
    #[error(transparent)]
    Wire(#[from] WireError),
}
