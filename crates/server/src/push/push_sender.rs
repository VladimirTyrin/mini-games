use super::{
    FcmConfigError, FcmSender, PushError, PushNotification, PushTarget, VapidKey, VapidKeyError,
    WebPushSender,
};

/// Every push credential this server has; a kind of target without them cannot be reached.
#[derive(Debug, Default)]
pub(crate) struct PushSender {
    fcm: Option<FcmSender>,
    web_push: Option<WebPushSender>,
}

#[derive(Debug, thiserror::Error)]
pub enum PushConfigError {
    #[error("FCM: {0}")]
    Fcm(#[source] FcmConfigError),
    #[error(transparent)]
    Vapid(VapidKeyError),
    #[error("MG_VAPID_SUBJECT must be an https:// or mailto: URL when MG_VAPID_PRIVATE_KEY is set")]
    VapidSubject,
}

impl PushSender {
    /// Empty settings leave that kind of push off.
    pub(crate) fn configure(
        fcm_service_account_base64: &str,
        vapid_private_key_hex: &str,
        vapid_subject: &str,
    ) -> Result<Self, PushConfigError> {
        let fcm = match fcm_service_account_base64.trim() {
            "" => None,
            account => Some(FcmSender::from_base64(account).map_err(PushConfigError::Fcm)?),
        };
        let web_push = match vapid_private_key_hex.trim() {
            "" => None,
            key => {
                let subject = vapid_subject.trim();
                if !(subject.starts_with("https://") || subject.starts_with("mailto:")) {
                    return Err(PushConfigError::VapidSubject);
                }
                let key = VapidKey::from_hex(key).map_err(PushConfigError::Vapid)?;
                Some(WebPushSender::new(key, subject.to_owned()))
            }
        };
        Ok(Self { fcm, web_push })
    }

    pub(crate) fn web_push_public_key(&self) -> Option<&str> {
        self.web_push.as_ref().map(WebPushSender::public_key)
    }

    pub(crate) fn can_reach(&self, target: &PushTarget) -> bool {
        match target {
            PushTarget::Fcm(_) => self.fcm.is_some(),
            PushTarget::WebPush(_) => self.web_push.is_some(),
        }
    }

    /// Blocking HTTP: call from a blocking task.
    pub(crate) fn send(
        &self,
        target: &PushTarget,
        notification: &PushNotification,
    ) -> Result<(), PushError> {
        match target {
            PushTarget::Fcm(token) => self
                .fcm
                .as_ref()
                .ok_or(PushError::Unavailable("FCM"))?
                .send(token, notification),
            PushTarget::WebPush(subscription) => self
                .web_push
                .as_ref()
                .ok_or(PushError::Unavailable("Web Push"))?
                .send(subscription, notification),
        }
    }
}
