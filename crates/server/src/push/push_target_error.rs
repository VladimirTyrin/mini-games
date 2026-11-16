#[derive(Debug, thiserror::Error)]
pub(crate) enum PushTargetError {
    #[error("no push target")]
    Missing,
    #[error("invalid FCM token")]
    FcmToken,
    #[error("push endpoint must be https at a known push service")]
    Endpoint,
    #[error("invalid p256dh key of the push subscription")]
    ClientKey,
    #[error("invalid auth secret of the push subscription")]
    AuthSecret,
}
