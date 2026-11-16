use crate::settings::{Dimensions, WinLength};

/// [`crate::Settings`] after validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rules {
    pub(crate) dimensions: Dimensions,
    pub(crate) win_length: WinLength,
}
