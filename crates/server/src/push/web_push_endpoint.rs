use axum::http::Uri;

use super::PushTargetError;

/// A push service URL from a browser's subscription. The server POSTs to it, so only https URLs
/// of the known push services pass: anything else would let a client aim the server's requests
/// at arbitrary hosts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WebPushEndpoint {
    url: String,
    /// `https://<host>`: the `aud` of the VAPID token.
    origin: String,
}

impl WebPushEndpoint {
    const MAX_CHARS: usize = 1024;
    /// Chrome and Edge (Android, desktop), Firefox, Safari (iOS home-screen apps, macOS),
    /// legacy Edge.
    const PUSH_SERVICE_HOSTS: [&'static str; 4] = [
        "fcm.googleapis.com",
        "push.services.mozilla.com",
        "push.apple.com",
        "notify.windows.com",
    ];

    pub(crate) fn parse(raw: String) -> Result<Self, PushTargetError> {
        if raw.len() > Self::MAX_CHARS {
            return Err(PushTargetError::Endpoint);
        }
        let uri: Uri = raw.parse().map_err(|_| PushTargetError::Endpoint)?;
        let authority = uri.authority().ok_or(PushTargetError::Endpoint)?;
        let host = authority.host().to_ascii_lowercase();
        let known_host = Self::PUSH_SERVICE_HOSTS.iter().any(|service| {
            host == *service
                || host
                    .strip_suffix(service)
                    .is_some_and(|prefix| prefix.ends_with('.'))
        });
        let plain_authority = authority.as_str().eq_ignore_ascii_case(&host);
        if uri.scheme_str() != Some("https") || !known_host || !plain_authority {
            return Err(PushTargetError::Endpoint);
        }
        Ok(Self {
            origin: format!("https://{host}"),
            url: raw,
        })
    }

    pub(crate) fn url(&self) -> &str {
        &self.url
    }

    pub(crate) fn origin(&self) -> &str {
        &self.origin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parses(url: &str) -> bool {
        WebPushEndpoint::parse(url.to_owned()).is_ok()
    }

    #[test]
    fn accepts_the_push_services_of_the_browsers() {
        assert!(parses("https://fcm.googleapis.com/fcm/send/abc:APA91b"));
        assert!(parses("https://web.push.apple.com/QGuR0mL6Kb"));
        assert!(parses(
            "https://updates.push.services.mozilla.com/wpush/v2/gAAAA"
        ));
        assert!(parses(
            "https://wns2-db5p.notify.windows.com/w/?token=BQYAAA"
        ));
    }

    #[test]
    fn origin_is_the_audience_of_the_push_service() {
        let endpoint = WebPushEndpoint::parse("https://web.push.apple.com/QGuR0mL6Kb".into());

        assert_eq!(endpoint.unwrap().origin(), "https://web.push.apple.com");
    }

    #[test]
    fn rejects_urls_that_would_send_requests_elsewhere() {
        assert!(!parses("http://fcm.googleapis.com/fcm/send/abc"));
        assert!(!parses("https://127.0.0.1:5000/ws"));
        assert!(!parses("https://localhost/push"));
        assert!(!parses("https://example.com/fcm.googleapis.com"));
        assert!(!parses("https://fcm.googleapis.com.example.com/x"));
        assert!(!parses("https://evilpush.apple.com/x"));
        assert!(!parses("https://fcm.googleapis.com@example.com/x"));
        assert!(!parses("https://fcm.googleapis.com:8443/x"));
        assert!(!parses("not a url"));
    }
}
