use std::collections::BTreeMap;
use std::time::Duration;

/// What a device shows, and what the app gets when it is opened from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PushNotification {
    pub(crate) title: String,
    pub(crate) body: String,
    /// A later notification with the same tag replaces this one on the device.
    pub(crate) tag: &'static str,
    /// Handed to the app when the notification is opened. Strings only: FCM data allows no
    /// other values.
    pub(crate) data: BTreeMap<&'static str, String>,
    /// Undelivered after this long, the push service drops it: an invitation is useless later.
    pub(crate) time_to_live: Duration,
}

impl PushNotification {
    /// What the service worker gets (and decrypts) for a Web Push.
    pub(crate) fn web_push_payload(&self) -> String {
        serde_json::json!({
            "title": self.title,
            "body": self.body,
            "tag": self.tag,
            "data": self.data,
        })
        .to_string()
    }
}
