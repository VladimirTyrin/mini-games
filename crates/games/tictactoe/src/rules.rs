use crate::dimensions::Dimensions;
use crate::win_length::WinLength;

/// [`crate::Settings`] after validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rules {
    pub(crate) dimensions: Dimensions,
    pub(crate) win_length: WinLength,
}
