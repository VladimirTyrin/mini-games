use super::web_push_encryption::EncryptionError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum PushError {
    #[error("this server has no credentials for {0}")]
    Unavailable(&'static str),
    #[error("encrypting the push message failed")]
    Encryption(#[source] EncryptionError),
    #[error("signing the FCM sign-in token failed")]
    Signing(#[source] jsonwebtoken::errors::Error),
    #[error("request to {service} failed")]
    Request {
        service: &'static str,
        #[source]
        source: ureq::Error,
    },
    #[error("{service} answered {status}: {detail}")]
    Rejected {
        service: &'static str,
        status: u16,
        detail: String,
    },
    /// The device unsubscribed or uninstalled the app: forget this target.
    #[error("the device is no longer subscribed")]
    TargetGone,
}
