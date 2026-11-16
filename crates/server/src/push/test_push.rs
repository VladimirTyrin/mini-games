use std::collections::BTreeMap;
use std::time::Duration;

use mg_core::Capped;

use super::PushNotification;

/// Seconds between the request and the test notification: time to close the app first.
pub(crate) type TestPushDelay = Capped<0, 60>;

/// The notification of "Test notifications" in the settings. `sentAt` lets the device tell how
/// long delivery took.
pub(crate) fn test_notification(sent_at_unix_ms: i64) -> PushNotification {
    PushNotification {
        title: "Mini Games".to_owned(),
        body: "Test notification: tap to see how long it took.".to_owned(),
        tag: "test".to_owned(),
        data: BTreeMap::from([
            ("kind", "test".to_owned()),
            ("sentAt", sent_at_unix_ms.to_string()),
        ]),
        time_to_live: Duration::from_secs(60),
    }
}
