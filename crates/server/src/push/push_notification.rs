use std::collections::BTreeMap;
use std::time::Duration;

use super::PushText;

/// What a device shows, and what the app gets when it is opened from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PushNotification {
    pub(crate) title: PushText,
    pub(crate) body: PushText,
    /// A later notification with the same tag replaces this one on the device.
    pub(crate) tag: String,
    /// Handed to the app when the notification is opened. Strings only: FCM data allows no
    /// other values.
    pub(crate) data: BTreeMap<&'static str, String>,
    /// Undelivered after this long, the push service drops it: an invitation is useless later.
    pub(crate) time_to_live: Duration,
}

impl PushNotification {
    /// What the service worker gets (and decrypts) for a Web Push: the keys it words, and the
    /// English for a worker from before them.
    pub(crate) fn web_push_payload(&self) -> String {
        serde_json::json!({
            "title": self.title.english(),
            "body": self.body.english(),
            "titleKey": self.title.key(),
            "titleArgs": self.title.args(),
            "bodyKey": self.body.key(),
            "bodyArgs": self.body.args(),
            "tag": self.tag,
            "data": self.data,
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_web_push_carries_the_keys_and_arguments_beside_the_english() {
        let notification = PushNotification {
            title: PushText::AppTitle,
            body: PushText::FriendRequest {
                from: "Sam".to_owned(),
            },
            tag: "friends".to_owned(),
            data: BTreeMap::from([("kind", "friends".to_owned())]),
            time_to_live: Duration::from_secs(60),
        };
        let payload: serde_json::Value =
            serde_json::from_str(&notification.web_push_payload()).expect("the payload is JSON");
        assert_eq!(
            payload,
            serde_json::json!({
                "title": "Mini Games",
                "body": "Sam wants to be your friend.",
                "titleKey": "push_title",
                "titleArgs": [],
                "bodyKey": "push_friend_request",
                "bodyArgs": ["Sam"],
                "tag": "friends",
                "data": { "kind": "friends" },
            })
        );
    }
}
