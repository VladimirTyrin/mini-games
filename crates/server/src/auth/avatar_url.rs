/// Profile picture URL shown to other players. Only Google-hosted HTTPS images are accepted, so
/// a client never loads an image from a host chosen by whoever produced the claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AvatarUrl(String);

impl AvatarUrl {
    pub(crate) const MAX_BYTES: usize = 512;
    const SCHEME: &'static str = "https://";
    const ALLOWED_DOMAIN: &'static str = "googleusercontent.com";

    /// `None` for anything that is not an acceptable avatar: the avatar is optional, so a bad
    /// URL is dropped rather than failing the sign-in.
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        if raw.len() > Self::MAX_BYTES
            || raw
                .chars()
                .any(|character| character.is_whitespace() || character.is_control())
        {
            return None;
        }
        let scheme_length = Self::SCHEME.len();
        let has_https_scheme = raw
            .get(..scheme_length)
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case(Self::SCHEME));
        if !has_https_scheme {
            return None;
        }
        let rest = raw.get(scheme_length..)?;
        let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
        // Userinfo or an explicit port would let the visible host differ from the real one.
        if authority.contains(['@', ':', '\\']) {
            return None;
        }
        let host = authority.to_ascii_lowercase();
        let host = host.strip_suffix('.').unwrap_or(&host);
        let is_google_hosted = host == Self::ALLOWED_DOMAIN
            || host
                .strip_suffix(Self::ALLOWED_DOMAIN)
                .and_then(|subdomain| subdomain.strip_suffix('.'))
                .is_some_and(|subdomain| !subdomain.is_empty() && !subdomain.ends_with('.'));
        is_google_hosted.then(|| Self(raw.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_google_hosted_https_images() {
        let url = "https://lh3.googleusercontent.com/a/ACg8ocJ=s96-c";
        assert_eq!(AvatarUrl::parse(url).unwrap().as_str(), url);
        assert!(AvatarUrl::parse("https://googleusercontent.com/a.png").is_some());
        assert!(AvatarUrl::parse("HTTPS://LH3.GoogleUserContent.com/a").is_some());
        assert!(AvatarUrl::parse("https://lh3.googleusercontent.com").is_some());
    }

    #[test]
    fn rejects_other_schemes_hosts_and_spoofing() {
        for url in [
            "",
            "http://lh3.googleusercontent.com/a",
            "javascript:alert(1)",
            "https://evil.example/a",
            "https://evilgoogleusercontent.com/a",
            "https://googleusercontent.com.evil.example/a",
            "https://lh3.googleusercontent.com@evil.example/a",
            "https://user@lh3.googleusercontent.com/a",
            "https://lh3.googleusercontent.com:8443/a",
            "https://.googleusercontent.com/a",
            "https://a..googleusercontent.com/a",
            "https://lh3.googleusercontent.com/a b",
            "https://lh3.googleusercontent.com/a\nb",
            "https://lh3.googleusercontent.com\\@evil.example/a",
            "//lh3.googleusercontent.com/a",
        ] {
            assert!(AvatarUrl::parse(url).is_none(), "{url:?} must be rejected");
        }
    }

    #[test]
    fn rejects_urls_over_the_size_limit() {
        let prefix = "https://lh3.googleusercontent.com/";
        let fits = format!(
            "{prefix}{}",
            "a".repeat(AvatarUrl::MAX_BYTES - prefix.len())
        );
        assert!(AvatarUrl::parse(&fits).is_some());
        assert!(AvatarUrl::parse(&format!("{fits}a")).is_none());
    }
}
