use super::PushTargetError;

/// An FCM registration token as the Android app reported it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FcmToken(String);

impl FcmToken {
    /// Real tokens are about 160 characters of `[A-Za-z0-9_:-]`.
    const MAX_CHARS: usize = 1024;

    pub(crate) fn parse(raw: String) -> Result<Self, PushTargetError> {
        let valid = !raw.is_empty()
            && raw.len() <= Self::MAX_CHARS
            && raw
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':'));
        if valid {
            Ok(Self(raw))
        } else {
            Err(PushTargetError::FcmToken)
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}
