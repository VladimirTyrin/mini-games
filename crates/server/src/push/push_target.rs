use mg_protocol::proto;
use proto::push_target::Target;

use super::{FcmToken, PushTargetError, WebPushSubscription};

/// Where one device receives push notifications.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PushTarget {
    Fcm(FcmToken),
    WebPush(WebPushSubscription),
}

impl PushTarget {
    pub(crate) fn parse(raw: Option<proto::PushTarget>) -> Result<Self, PushTargetError> {
        match raw.and_then(|target| target.target) {
            Some(Target::FcmToken(token)) => FcmToken::parse(token).map(Self::Fcm),
            Some(Target::WebPush(subscription)) => {
                WebPushSubscription::parse(subscription).map(Self::WebPush)
            }
            None => Err(PushTargetError::Missing),
        }
    }
}
